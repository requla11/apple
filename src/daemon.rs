use crate::audit::{AuditStore, audit_record_path};
use crate::isolation::{
    HermeticEnvironmentSanitizer, HermeticFilesystemManager, NetworkIsolationController,
    ProcessIsolationRunner,
};
use crate::protocol::{DaemonMessage, ExecutionRequest, ExecutionResult};
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct AppleDaemonServer {
    scratch_dir: PathBuf,
    active_sandboxes: Arc<AtomicUsize>,
    is_running: Arc<AtomicBool>,
    audit_store: AuditStore,
}

impl AppleDaemonServer {
    pub fn new(scratch_dir: PathBuf) -> Self {
        Self {
            scratch_dir,
            active_sandboxes: Arc::new(AtomicUsize::new(0)),
            is_running: Arc::new(AtomicBool::new(true)),
            audit_store: AuditStore::new(),
        }
    }

    pub fn audit_store(&self) -> &AuditStore {
        &self.audit_store
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Serve IPC requests on `endpoint` (a filesystem path on Unix, a named
    /// pipe name on Windows). Messages are newline-delimited JSON
    /// (`DaemonMessage`). Returns when a `Shutdown` message is received or
    /// Ctrl+C is pressed.
    pub async fn serve(self: Arc<Self>, endpoint: &str) -> Result<()> {
        #[cfg(unix)]
        {
            let listener = tokio::net::UnixListener::bind(endpoint)?;
            loop {
                if !self.is_running() {
                    break;
                }
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => break,
                    accepted = listener.accept() => {
                        let (stream, _) = accepted?;
                        let server = self.clone();
                        tokio::spawn(async move {
                            let _ = server.handle_connection(stream).await;
                        });
                    }
                }
            }
            Ok(())
        }

        #[cfg(windows)]
        {
            use tokio::net::windows::named_pipe::ServerOptions;
            let pipe_name = normalize_windows_pipe(endpoint);
            let mut server = ServerOptions::new()
                .first_pipe_instance(true)
                .create(&pipe_name)?;
            loop {
                if !self.is_running() {
                    break;
                }
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => break,
                    connected = server.connect() => {
                        connected?;
                        // Queue the next pipe instance for the following client.
                        let next = ServerOptions::new().create(&pipe_name)?;
                        let current = server;
                        server = next;
                        let srv = self.clone();
                        tokio::spawn(async move {
                            let _ = srv.handle_connection(current).await;
                        });
                    }
                }
            }
            Ok(())
        }
    }

    /// Send a `Ping` to a daemon listening on `endpoint`. Returns the
    /// daemon's version and active sandbox count if reachable.
    pub async fn ping_endpoint(endpoint: &str) -> Result<(String, usize)> {
        #[cfg(unix)]
        {
            use tokio::net::UnixStream;
            let stream = UnixStream::connect(endpoint).await?;
            ping_over(stream).await
        }

        #[cfg(windows)]
        {
            use tokio::net::windows::named_pipe::ClientOptions;
            let pipe_name = normalize_windows_pipe(endpoint);
            let stream = ClientOptions::new().open(&pipe_name)?;
            ping_over(stream).await
        }
    }

    async fn handle_connection<S>(&self, stream: S) -> Result<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        let (reader, mut writer) = tokio::io::split(stream);
        let mut lines = BufReader::new(reader).lines();

        while let Some(line) = lines.next_line().await? {
            let message = match serde_json::from_str::<DaemonMessage>(&line) {
                Ok(msg) => msg,
                Err(_) => break, // malformed frame; close the connection
            };

            if matches!(message, DaemonMessage::Shutdown) {
                self.is_running.store(false, Ordering::SeqCst);
                let reply = DaemonMessage::Pong {
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    active_sandboxes: self.active_sandboxes.load(Ordering::Relaxed),
                };
                write_frame(&mut writer, &reply).await?;
                break;
            }

            let reply = self.dispatch_message(message).await;
            write_frame(&mut writer, &reply).await?;
        }
        Ok(())
    }

    pub async fn dispatch_message(&self, message: DaemonMessage) -> DaemonMessage {
        match message {
            DaemonMessage::Ping => DaemonMessage::Pong {
                version: env!("CARGO_PKG_VERSION").to_string(),
                active_sandboxes: self.active_sandboxes.load(Ordering::Relaxed),
            },
            DaemonMessage::Execute(request) => {
                let result = self.execute_task(request).await;
                DaemonMessage::Result(result)
            }
            DaemonMessage::Cancel { .. } => DaemonMessage::Ping,
            DaemonMessage::Shutdown => {
                self.is_running.store(false, Ordering::SeqCst);
                DaemonMessage::Pong {
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    active_sandboxes: 0,
                }
            }
            other => other,
        }
    }

    pub async fn execute_task(&self, mut request: ExecutionRequest) -> ExecutionResult {
        self.active_sandboxes.fetch_add(1, Ordering::SeqCst);

        let fs_mgr = HermeticFilesystemManager::new(&self.scratch_dir);

        // Prepare the jail first so the per-task scratch tmp directory exists
        // and can be handed to the environment sanitizer as an absolute path.
        let jail_dir =
            match fs_mgr.prepare_workspace_jail(&request.task_id, &request.profile.mount_rules) {
                Ok(jail) => {
                    let _ = std::fs::create_dir_all(jail.join("tmp"));
                    Some(jail)
                }
                Err(_) => None,
            };

        let tmp_dir: Option<&Path> = jail_dir.as_deref();
        let mut sanitized_env = HermeticEnvironmentSanitizer::sanitize(
            &request.env,
            &request.profile.whitelisted_env,
            tmp_dir,
        );
        NetworkIsolationController::apply_network_policy(
            &mut sanitized_env,
            request.profile.allow_network,
        );
        request.env = sanitized_env;

        if let Some(jail) = &jail_dir {
            request.working_dir = jail.clone();
        }

        let exec_res = ProcessIsolationRunner::run_sandboxed(request.clone()).await;
        if !request.keep_jail {
            let _ = fs_mgr.cleanup_jail(&request.task_id);
        }

        self.active_sandboxes.fetch_sub(1, Ordering::SeqCst);

        let final_result = match exec_res {
            Ok(result) => result,
            Err(err) => ExecutionResult {
                task_id: request.task_id,
                exit_code: 1,
                stdout: Vec::new(),
                stderr: err.to_string().into_bytes(),
                execution_duration_ms: 0,
                peak_memory_bytes: 0,
                violations: Vec::new(),
                hermetic_guarantee: false,
            },
        };

        self.audit_store.record_result(&final_result);
        self.persist_audit(&final_result);
        final_result
    }

    /// Persist the execution result as JSON under
    /// `<scratch>/audit/<task_id>.json` so other processes (e.g. the CLI)
    /// can inspect real audit data instead of placeholder output.
    fn persist_audit(&self, result: &ExecutionResult) {
        let dir = self.scratch_dir.join("audit");
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let path = audit_record_path(&self.scratch_dir, &result.task_id);
        if let Ok(body) = serde_json::to_string_pretty(result) {
            let _ = std::fs::write(path, body);
        }
    }
}

async fn write_frame<W>(writer: &mut W, reply: &DaemonMessage) -> Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut frame = serde_json::to_vec(reply)?;
    frame.push(b'\n');
    writer.write_all(&frame).await?;
    writer.flush().await?;
    Ok(())
}

async fn ping_over<S>(stream: S) -> Result<(String, usize)>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    use tokio::io::AsyncWriteExt as _;
    let (reader, mut writer) = tokio::io::split(stream);
    writer.write_all(b"{\"Ping\":null}\n").await?;
    writer.flush().await?;
    drop(writer);

    let mut lines = BufReader::new(reader).lines();
    let line = lines
        .next_line()
        .await?
        .ok_or_else(|| anyhow::anyhow!("daemon closed the connection without a reply"))?;
    match serde_json::from_str::<DaemonMessage>(&line)? {
        DaemonMessage::Pong {
            version,
            active_sandboxes,
        } => Ok((version, active_sandboxes)),
        _ => anyhow::bail!("unexpected reply from daemon"),
    }
}

#[cfg(windows)]
fn normalize_windows_pipe(endpoint: &str) -> String {
    if endpoint.starts_with(r"\\.\pipe\") {
        endpoint.to_string()
    } else {
        format!(r"\\.\pipe\{endpoint}")
    }
}
