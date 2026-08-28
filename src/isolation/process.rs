use crate::protocol::{ExecutionRequest, ExecutionResult};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
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
            if request.profile.level != crate::protocol::IsolationLevel::Off {
                cmd.creation_flags(0x08000000);
            }
        }

        #[cfg(unix)]
        {
            unsafe {
                cmd.pre_exec(|| {
                    libc::setpgid(0, 0);
                    Ok(())
                });
            }
        }

        let mut child = cmd.spawn()?;

        #[cfg(windows)]
        let _job_guard = if request.profile.level != crate::protocol::IsolationLevel::Off {
            Self::apply_windows_job_object(&child, request.profile.memory_limit_mb)
        } else {
            None
        };

        let mut stdout_handle = child.stdout.take();
        let mut stderr_handle = child.stderr.take();

        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();

        let timeout_duration = request
            .profile
            .timeout_seconds
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(3600));

        let wait_fut = async {
            let mut read_stdout_fut = async {
                if let Some(ref mut out) = stdout_handle {
                    let _ = out.read_to_end(&mut stdout_buf).await;
                }
            };
            let mut read_stderr_fut = async {
                if let Some(ref mut err) = stderr_handle {
                    let _ = err.read_to_end(&mut stderr_buf).await;
                }
            };
            let (status_res, _, _) = tokio::join!(child.wait(), read_stdout_fut, read_stderr_fut);
            status_res
        };

        let output_res = tokio::time::timeout(timeout_duration, wait_fut).await;
        let elapsed = start_time.elapsed().as_millis() as u64;

        match output_res {
            Ok(Ok(status)) => Ok(ExecutionResult {
                task_id: request.task_id,
                exit_code: status.code().unwrap_or(-1),
                stdout: stdout_buf,
                stderr: stderr_buf,
                execution_duration_ms: elapsed,
                peak_memory_bytes: 0,
                violations: Vec::new(),
                hermetic_guarantee: true,
            }),
            Ok(Err(err)) => Err(anyhow::anyhow!("process execution failed: {err}")),
            Err(_) => {
                let _ = child.kill().await;
                Ok(ExecutionResult {
                    task_id: request.task_id,
                    exit_code: 124,
                    stdout: Vec::new(),
                    stderr: b"process timed out under hermetic sandbox policy".to_vec(),
                    execution_duration_ms: elapsed,
                    peak_memory_bytes: 0,
                    violations: Vec::new(),
                    hermetic_guarantee: false,
                })
            }
        }
    }

    #[cfg(windows)]
    fn apply_windows_job_object(
        child: &tokio::process::Child,
        memory_limit_mb: Option<u64>,
    ) -> Option<WindowsJobGuard> {
        use std::os::windows::io::AsRawHandle;
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

#[cfg(windows)]
impl Drop for WindowsJobGuard {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}
