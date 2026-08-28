use crate::daemon::AppleDaemonServer;
use crate::protocol::{DaemonMessage, ExecutionRequest, ExecutionResult};
use std::sync::Arc;

pub struct AppleClient {
    server: Arc<AppleDaemonServer>,
}

impl AppleClient {
    pub fn new(server: Arc<AppleDaemonServer>) -> Self {
        Self { server }
    }

    pub async fn ping(&self) -> bool {
        matches!(
            self.server.dispatch_message(DaemonMessage::Ping).await,
            DaemonMessage::Pong { .. }
        )
    }

    pub async fn execute(&self, request: ExecutionRequest) -> ExecutionResult {
        match self
            .server
            .dispatch_message(DaemonMessage::Execute(request))
            .await
        {
            DaemonMessage::Result(result) => result,
            _ => ExecutionResult {
                task_id: "unknown".to_string(),
                exit_code: -1,
                stdout: Vec::new(),
                stderr: b"daemon communication failure".to_vec(),
                execution_duration_ms: 0,
                peak_memory_bytes: 0,
                violations: Vec::new(),
                hermetic_guarantee: false,
            },
        }
    }
}
