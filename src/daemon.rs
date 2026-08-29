use crate::audit::{AuditStore, audit_record_path};
use crate::isolation::{
    AmbientDaemonScrubber, HermeticEnvironmentSanitizer, HermeticFilesystemManager,
    HermeticToolchainSanitizer, NetworkIsolationController, NumaAffinityController,
    ProcessIsolationRunner,
};
use crate::protocol::{DaemonMessage, ExecutionRequest, ExecutionResult};

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct AppleDaemonServer {
    scratch_dir: PathBuf,
    active_sandboxes: Arc<AtomicUsize>,
    is_running: Arc<AtomicBool>,
    audit_store: AuditStore,
    cancel_handles: Arc<Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<()>>>>,
}

impl AppleDaemonServer {
    pub fn new(scratch_dir: PathBuf) -> Self {
        Self {
            scratch_dir,
            active_sandboxes: Arc::new(AtomicUsize::new(0)),
            is_running: Arc::new(AtomicBool::new(true)),
            audit_store: AuditStore::new(),
            cancel_handles: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn audit_store(&self) -> &AuditStore {
        &self.audit_store
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    pub fn cancel_task(&self, task_id: &str) -> bool {
        let Ok(mut handles) = self.cancel_handles.lock() else {
            return false;
        };
        if let Some(tx) = handles.remove(task_id) {
            let _ = tx.send(());
            true
        } else {
            false
        }
    }

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
                Err(_) => break,
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
            DaemonMessage::Cancel { task_id } => {
                let _ = self.cancel_task(&task_id);
                DaemonMessage::Cancelled { task_id }
            }
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

        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
        if let Ok(mut handles) = self.cancel_handles.lock() {
            handles.insert(request.task_id.clone(), cancel_tx);
        }

        let fs_mgr = HermeticFilesystemManager::new(&self.scratch_dir);

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
        AmbientDaemonScrubber::scrub_ambient_env(&mut sanitized_env);
        HermeticToolchainSanitizer::inject_deterministic_flags(
            &mut sanitized_env,
            jail_dir.as_deref(),
        );
        if let Some(node) = request.profile.numa_node {
            NumaAffinityController::assign_numa_node(node, &mut sanitized_env);
        }
        request.env = sanitized_env;

        if let Some(jail) = &jail_dir {
            request.working_dir = jail.clone();
        }

        let exec_res = ProcessIsolationRunner::run_sandboxed_cancellable_streamed(
            request.clone(),
            Some(cancel_rx),
            None,
        )
        .await;

        if let Ok(mut handles) = self.cancel_handles.lock() {
            handles.remove(&request.task_id);
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_daemon_dispatch_ping_pong() {
        let temp = TempDir::new().unwrap();
        let server = AppleDaemonServer::new(temp.path().to_path_buf());
        let reply = server.dispatch_message(DaemonMessage::Ping).await;
        if let DaemonMessage::Pong {
            version,
            active_sandboxes,
        } = reply
        {
            assert_eq!(version, env!("CARGO_PKG_VERSION"));
            assert_eq!(active_sandboxes, 0);
        } else {
            panic!("expected Pong");
        }
    }

    #[tokio::test]
    async fn test_daemon_dispatch_cancel() {
        let temp = TempDir::new().unwrap();
        let server = AppleDaemonServer::new(temp.path().to_path_buf());
        let reply = server
            .dispatch_message(DaemonMessage::Cancel {
                task_id: "test_task_cancel".to_string(),
            })
            .await;
        assert_eq!(
            reply,
            DaemonMessage::Cancelled {
                task_id: "test_task_cancel".to_string(),
            }
        );
    }
}
