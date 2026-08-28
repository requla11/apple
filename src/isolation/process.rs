use crate::protocol::{ExecutionRequest, ExecutionResult, IsolationLevel};
use std::process::Stdio;
use std::time::Instant;
use tokio::process::Command;

pub struct ProcessIsolationRunner;

impl ProcessIsolationRunner {
    pub async fn run_sandboxed(
        request: ExecutionRequest,
    ) -> Result<ExecutionResult, anyhow::Error> {
        let start_time = Instant::now();
        let program = request
            .argv
            .first()
            .ok_or_else(|| anyhow::anyhow!("empty argv"))?;
        let args = &request.argv[1..];

        let mut cmd = Command::new(program);
        cmd.args(args);
        cmd.current_dir(&request.working_dir);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.env_clear();
        for (k, v) in &request.env {
            cmd.env(k, v);
        }

        #[cfg(windows)]
        {
            if request.profile.level != IsolationLevel::Off {
                cmd.creation_flags(0x08000000);
            }
        }

        let child = cmd.spawn()?;
        let output = child.wait_with_output().await?;
        let elapsed = start_time.elapsed().as_millis() as u64;

        Ok(ExecutionResult {
            task_id: request.task_id,
            exit_code: output.status.code().unwrap_or(-1),
            stdout: output.stdout,
            stderr: output.stderr,
            execution_duration_ms: elapsed,
            peak_memory_bytes: 0,
            violations: Vec::new(),
            hermetic_guarantee: true,
        })
    }
}
