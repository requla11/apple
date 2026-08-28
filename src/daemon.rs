use crate::isolation::{
    HermeticEnvironmentSanitizer, HermeticFilesystemManager, NetworkIsolationController,
    ProcessIsolationRunner,
};
use crate::protocol::{DaemonMessage, ExecutionRequest, ExecutionResult};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub struct AppleDaemonServer {
    scratch_dir: PathBuf,
    active_sandboxes: Arc<AtomicUsize>,
    is_running: Arc<AtomicBool>,
}

impl AppleDaemonServer {
    pub fn new(scratch_dir: PathBuf) -> Self {
        Self {
            scratch_dir,
            active_sandboxes: Arc::new(AtomicUsize::new(0)),
            is_running: Arc::new(AtomicBool::new(true)),
        }
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

    async fn execute_task(&self, mut request: ExecutionRequest) -> ExecutionResult {
        self.active_sandboxes.fetch_add(1, Ordering::SeqCst);
        let fs_mgr = HermeticFilesystemManager::new(&self.scratch_dir);

        let mut sanitized_env =
            HermeticEnvironmentSanitizer::sanitize(&request.env, &request.profile.whitelisted_env);
        NetworkIsolationController::apply_network_policy(
            &mut sanitized_env,
            request.profile.allow_network,
        );
        request.env = sanitized_env;

        let jail_res =
            fs_mgr.prepare_workspace_jail(&request.task_id, &request.profile.mount_rules);
        if let Ok(jail_path) = jail_res {
            request.working_dir = jail_path;
        }

        let exec_res = ProcessIsolationRunner::run_sandboxed(request.clone()).await;
        let _ = fs_mgr.cleanup_jail(&request.task_id);

        self.active_sandboxes.fetch_sub(1, Ordering::SeqCst);

        match exec_res {
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
        }
    }
}
