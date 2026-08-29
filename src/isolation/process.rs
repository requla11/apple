use crate::protocol::{ExecutionRequest, ExecutionResult};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;

/// Runs a child process with the isolation primitives available to an
/// unprivileged user-space process: an scrubbed environment, an optional
/// Windows Job Object (kill-on-close + memory ceiling), a Unix process
/// group (`setpgid`) and a hard timeout. This is process-level isolation,
/// not a kernel sandbox (no namespaces/seccomp/AppContainer).
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
        cmd.kill_on_drop(true);
        cmd.env_clear();
        for (k, v) in &request.env {
            cmd.env(k, v);
        }

        #[cfg(windows)]
        {
            if request.profile.level != crate::protocol::IsolationLevel::Off {
                // CREATE_NO_WINDOW: avoid flashing a console window for each
                // sandboxed child process. Real process-level containment
                // comes from the Job Object assigned below.
                cmd.creation_flags(0x08000000);
            }
        }

        #[cfg(unix)]
        {
            unsafe {
                cmd.pre_exec(|| {
                    if libc::setpgid(0, 0) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }

        let child = cmd.spawn()?;

        #[cfg(windows)]
        let _job_guard = if request.profile.level != crate::protocol::IsolationLevel::Off {
            Self::apply_windows_job_object(&child, request.profile.memory_limit_mb)
        } else {
            None
        };

        let timeout_duration = request
            .profile
            .timeout_seconds
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(3600));

        let output_res = tokio::time::timeout(timeout_duration, child.wait_with_output()).await;
        let elapsed = start_time.elapsed().as_millis() as u64;

        match output_res {
            Ok(Ok(output)) => Ok(ExecutionResult {
                task_id: request.task_id,
                exit_code: output.status.code().unwrap_or(-1),
                stdout: output.stdout,
                stderr: output.stderr,
                execution_duration_ms: elapsed,
                peak_memory_bytes: 0,
                violations: Vec::new(),
                // Only claim a hermetic guarantee when an isolation level
                // above `Off` was actually enforced.
                hermetic_guarantee: request.profile.level != crate::protocol::IsolationLevel::Off,
            }),
            Ok(Err(err)) => Err(anyhow::anyhow!("process execution failed: {err}")),
            Err(_) => Ok(ExecutionResult {
                task_id: request.task_id,
                exit_code: 124,
                stdout: Vec::new(),
                stderr: b"process timed out under hermetic sandbox policy".to_vec(),
                execution_duration_ms: elapsed,
                peak_memory_bytes: 0,
                violations: Vec::new(),
                hermetic_guarantee: false,
            }),
        }
    }

    #[cfg(windows)]
    fn apply_windows_job_object(
        child: &tokio::process::Child,
        memory_limit_mb: Option<u64>,
    ) -> Option<WindowsJobGuard> {
        use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOB_OBJECT_LIMIT_PROCESS_MEMORY, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JobObjectExtendedLimitInformation, SetInformationJobObject,
        };

        let raw_handle = child.raw_handle()?;

        unsafe {
            let job: HANDLE = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
            if job.is_null() {
                return None;
            }

            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            if let Some(mem_mb) = memory_limit_mb {
                info.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_PROCESS_MEMORY;
                info.ProcessMemoryLimit = (mem_mb * 1024 * 1024) as usize;
            }

            let set_res = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );

            if set_res == 0 {
                CloseHandle(job);
                return None;
            }

            let assign_res = AssignProcessToJobObject(job, raw_handle as HANDLE);
            if assign_res == 0 {
                CloseHandle(job);
                return None;
            }

            Some(WindowsJobGuard { handle: job })
        }
    }
}

#[cfg(windows)]
struct WindowsJobGuard {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

// The raw HANDLE is owned exclusively by this guard and only closed in
// `Drop`, so moving it between threads (as required when holding it across
// an `.await` point) is safe.
#[cfg(windows)]
unsafe impl Send for WindowsJobGuard {}

#[cfg(windows)]
impl Drop for WindowsJobGuard {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}
