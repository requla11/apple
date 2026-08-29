use crate::isolation::interceptor::LiveIoInterceptor;
use crate::isolation::macos::SeatbeltProfileBuilder;
use crate::protocol::{ExecutionRequest, ExecutionResult, IsolationLevel};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;

use tokio::io::AsyncReadExt;

pub struct ProcessIsolationRunner;

impl ProcessIsolationRunner {
    pub async fn run_sandboxed(
        request: ExecutionRequest,
    ) -> Result<ExecutionResult, anyhow::Error> {
        Self::run_sandboxed_cancellable_streamed(request, None, None).await
    }

    pub async fn run_sandboxed_cancellable_streamed(
        request: ExecutionRequest,
        mut cancel_rx: Option<tokio::sync::oneshot::Receiver<()>>,
        chunk_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::protocol::DaemonMessage>>,
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

        let mut child = cmd.spawn()?;

        #[cfg(windows)]
        let _job_guard = if request.profile.level != IsolationLevel::Off {
            Self::apply_windows_job_object(&child, request.profile.memory_limit_mb)
        } else {
            None
        };

        let stdout_pipe = child.stdout.take();
        let stderr_pipe = child.stderr.take();

        let task_id = request.task_id.clone();
        let chunk_tx_out = chunk_tx.clone();
        let task_id_out = task_id.clone();
        let stdout_task = tokio::spawn(async move {
            let mut accum = Vec::new();
            if let Some(mut pipe) = stdout_pipe {
                let mut buf = [0u8; 4096];
                while let Ok(n) = pipe.read(&mut buf).await {
                    if n == 0 {
                        break;
                    }
                    accum.extend_from_slice(&buf[..n]);
                    if let Some(ref tx) = chunk_tx_out {
                        let _ = tx.send(crate::protocol::DaemonMessage::StdoutChunk {
                            task_id: task_id_out.clone(),
                            data: buf[..n].to_vec(),
                        });
                    }
                }
            }
            accum
        });

        let chunk_tx_err = chunk_tx;
        let task_id_err = task_id.clone();
        let stderr_task = tokio::spawn(async move {
            let mut accum = Vec::new();
            if let Some(mut pipe) = stderr_pipe {
                let mut buf = [0u8; 4096];
                while let Ok(n) = pipe.read(&mut buf).await {
                    if n == 0 {
                        break;
                    }
                    accum.extend_from_slice(&buf[..n]);
                    if let Some(ref tx) = chunk_tx_err {
                        let _ = tx.send(crate::protocol::DaemonMessage::StderrChunk {
                            task_id: task_id_err.clone(),
                            data: buf[..n].to_vec(),
                        });
                    }
                }
            }
            accum
        });

        let timeout_duration = request
            .profile
            .timeout_seconds
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(3600));

        let _child_pid = child.id();
        let mut was_cancelled = false;
        let mut was_timeout = false;

        let status_res = tokio::select! {
            res = child.wait() => {
                res.map_err(|e| anyhow::anyhow!("child wait failed: {e}"))
            }
            _ = tokio::time::sleep(timeout_duration) => {
                was_timeout = true;
                let _ = child.start_kill();
                #[cfg(unix)]
                if let Some(pid) = _child_pid {
                    unsafe { libc::killpg(pid as i32, libc::SIGKILL); }
                }
                Err(anyhow::anyhow!("timeout"))
            }
            _ = async {
                if let Some(ref mut rx) = cancel_rx {
                    let _ = rx.await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {
                was_cancelled = true;
                let _ = child.start_kill();
                #[cfg(unix)]
                if let Some(pid) = _child_pid {
                    unsafe { libc::killpg(pid as i32, libc::SIGKILL); }
                }
                Err(anyhow::anyhow!("cancelled"))
            }
        };

        let stdout_data = stdout_task.await.unwrap_or_default();
        let stderr_data = stderr_task.await.unwrap_or_default();
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

        if was_cancelled {
            return Ok(ExecutionResult {
                task_id: request.task_id,
                exit_code: 130,
                stdout: stdout_data,
                stderr: b"process cancelled by daemon request".to_vec(),
                execution_duration_ms: elapsed,
                peak_memory_bytes: peak_mem,
                violations,
                hermetic_guarantee: false,
            });
        }

        if was_timeout {
            return Ok(ExecutionResult {
                task_id: request.task_id,
                exit_code: 124,
                stdout: stdout_data,
                stderr: b"process timed out under hermetic sandbox policy".to_vec(),
                execution_duration_ms: elapsed,
                peak_memory_bytes: peak_mem,
                violations,
                hermetic_guarantee: false,
            });
        }

        let status = status_res?;
        Ok(ExecutionResult {
            task_id: request.task_id,
            exit_code: status.code().unwrap_or(-1),
            stdout: stdout_data,
            stderr: stderr_data,
            execution_duration_ms: elapsed,
            peak_memory_bytes: peak_mem,
            violations,
            hermetic_guarantee: request.profile.level != IsolationLevel::Off,
        })
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
                max_processes: None,
                numa_node: None,
            },

            keep_jail: true,
        };

        let res = ProcessIsolationRunner::run_sandboxed(req).await.unwrap();
        assert_eq!(res.exit_code, 0);
        let stdout_str = String::from_utf8_lossy(&res.stdout);
        assert!(stdout_str.contains("hello_apple_sandbox"));
        assert!(res.hermetic_guarantee);
    }

    #[tokio::test]
    async fn test_process_isolation_runner_streaming() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let req = ExecutionRequest {
            task_id: "test_stream".to_string(),
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
                "echo stream_test_output".to_string(),
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
                max_processes: None,
                numa_node: None,
            },
            keep_jail: true,
        };

        let res = ProcessIsolationRunner::run_sandboxed_cancellable_streamed(req, None, Some(tx))
            .await
            .unwrap();
        assert_eq!(res.exit_code, 0);

        let mut received_chunks = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            if let crate::protocol::DaemonMessage::StdoutChunk { data, .. } = msg {
                received_chunks.extend_from_slice(&data);
            }
        }
        let streamed_str = String::from_utf8_lossy(&received_chunks);
        assert!(streamed_str.contains("stream_test_output"));
    }

    #[tokio::test]
    async fn test_process_isolation_runner_cancellation() {
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
        let req = ExecutionRequest {
            task_id: "test_cancel".to_string(),
            working_dir: std::env::temp_dir(),
            argv: vec![
                if cfg!(windows) {
                    "ping".to_string()
                } else {
                    "sleep".to_string()
                },
                if cfg!(windows) {
                    "127.0.0.1".to_string()
                } else {
                    "10".to_string()
                },
                if cfg!(windows) {
                    "-n".to_string()
                } else {
                    "".to_string()
                },
                if cfg!(windows) {
                    "10".to_string()
                } else {
                    "".to_string()
                },
            ]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect(),
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
                max_processes: None,
                numa_node: None,
            },
            keep_jail: true,
        };

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            let _ = cancel_tx.send(());
        });

        let res =
            ProcessIsolationRunner::run_sandboxed_cancellable_streamed(req, Some(cancel_rx), None)
                .await
                .unwrap();
        assert_eq!(res.exit_code, 130);
    }
}
