use crate::isolation::interceptor::LiveIoInterceptor;
use crate::isolation::macos::SeatbeltProfileBuilder;
use crate::protocol::{ExecutionRequest, ExecutionResult, IsolationLevel};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;

pub struct ProcessIsolationRunner;

impl ProcessIsolationRunner {
    pub async fn run_sandboxed(
        request: ExecutionRequest,
    ) -> Result<ExecutionResult, anyhow::Error> {
        let start_time = Instant::now();
        let mut violations = Vec::new();

        let interceptor = LiveIoInterceptor::new(
            vec![request.working_dir.clone(), std::env::temp_dir()],
            request.profile.declared_inputs.clone(),
        );

        for input in &request.profile.declared_inputs {
            if let Some(v) = interceptor.inspect_path_access(input, false) {
                violations.push(v);
            }
        }

        let program_argv =
            if cfg!(target_os = "macos") && request.profile.level == IsolationLevel::FullHermetic {
                let read_paths: Vec<PathBuf> = request
                    .profile
                    .mount_rules
                    .iter()
                    .map(|m| m.source.clone())
                    .collect();
                let write_paths = vec![request.working_dir.join("target")];
                let sbpl = SeatbeltProfileBuilder::build_sbpl_profile(
                    &request.working_dir,
                    &read_paths,
                    &write_paths,
                    request.profile.allow_network,
                );
                SeatbeltProfileBuilder::wrap_command(&request.argv, &sbpl)
            } else {
                request.argv.clone()
            };

        let program = program_argv
            .first()
            .ok_or_else(|| anyhow::anyhow!("empty argv"))?;
        let args = &program_argv[1..];

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
            if request.profile.level != IsolationLevel::Off {
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
        let _job_guard = if request.profile.level != IsolationLevel::Off {
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

        let peak_mem = {
            #[cfg(windows)]
            {
                _job_guard
                    .as_ref()
                    .map(|g| g.get_peak_memory_bytes())
                    .unwrap_or(0)
            }
            #[cfg(unix)]
            {
                Self::get_rusage_peak_memory()
            }
            #[cfg(not(any(windows, unix)))]
            {
                0
            }
        };

        match output_res {
            Ok(Ok(output)) => Ok(ExecutionResult {
                task_id: request.task_id,
                exit_code: output.status.code().unwrap_or(-1),
                stdout: output.stdout,
                stderr: output.stderr,
                execution_duration_ms: elapsed,
                peak_memory_bytes: peak_mem,
                violations,
                hermetic_guarantee: request.profile.level != IsolationLevel::Off,
            }),
            Ok(Err(err)) => Err(anyhow::anyhow!("process execution failed: {err}")),
            Err(_) => Ok(ExecutionResult {
                task_id: request.task_id,
                exit_code: 124,
                stdout: Vec::new(),
                stderr: b"process timed out under hermetic sandbox policy".to_vec(),
                execution_duration_ms: elapsed,
                peak_memory_bytes: peak_mem,
                violations,
                hermetic_guarantee: false,
            }),
        }
    }

    #[cfg(unix)]
    fn get_rusage_peak_memory() -> u64 {
        unsafe {
            let mut usage: libc::rusage = std::mem::zeroed();
            if libc::getrusage(libc::RUSAGE_CHILDREN, &mut usage) == 0 {
                #[cfg(target_os = "linux")]
                {
                    (usage.ru_maxrss as u64) * 1024
                }
                #[cfg(not(target_os = "linux"))]
                {
                    usage.ru_maxrss as u64
                }
            } else {
                0
            }
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

#[cfg(windows)]
impl WindowsJobGuard {
    pub fn get_peak_memory_bytes(&self) -> u64 {
        use windows_sys::Win32::System::JobObjects::{
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            QueryInformationJobObject,
        };
        unsafe {
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            let mut return_length: u32 = 0;
            let res = QueryInformationJobObject(
                self.handle,
                JobObjectExtendedLimitInformation,
                &mut info as *mut _ as *mut _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                &mut return_length,
            );
            if res != 0 {
                info.PeakJobMemoryUsed as u64
            } else {
                0
            }
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{IsolationLevel, SandboxProfile};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_process_isolation_runner_echo() {
        let req = ExecutionRequest {
            task_id: "test_echo".to_string(),
            working_dir: std::env::temp_dir(),
            argv: vec![
                if cfg!(windows) {
                    "cmd".to_string()
                } else {
                    "sh".to_string()
                },
                if cfg!(windows) {
                    "/C".to_string()
                } else {
                    "-c".to_string()
                },
                "echo hello_apple_sandbox".to_string(),
            ],
            env: HashMap::new(),
            profile: SandboxProfile {
                name: "test".to_string(),
                level: IsolationLevel::ProcessOnly,
                allow_network: false,
                memory_limit_mb: Some(512),
                cpu_affinity_mask: None,
                timeout_seconds: Some(10),
                mount_rules: Vec::new(),
                whitelisted_env: Vec::new(),
                seccomp_filter: true,
                appcontainer: false,
                declared_inputs: Vec::new(),
            },
            keep_jail: true,
        };

        let res = ProcessIsolationRunner::run_sandboxed(req).await.unwrap();
        assert_eq!(res.exit_code, 0);
        let stdout_str = String::from_utf8_lossy(&res.stdout);
        assert!(stdout_str.contains("hello_apple_sandbox"));
        assert!(res.hermetic_guarantee);
    }
}
