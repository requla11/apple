use std::path::Path;

pub struct ProcessLimitController;

impl ProcessLimitController {
    pub fn apply_cgroup_pid_limit(
        cgroup_root: &Path,
        max_processes: u32,
    ) -> Result<(), std::io::Error> {
        #[cfg(target_os = "linux")]
        {
            if cgroup_root.exists() {
                std::fs::write(cgroup_root.join("pids.max"), max_processes.to_string())?;
            }
        }
        let _ = (cgroup_root, max_processes);
        Ok(())
    }

    pub fn compute_default_pid_limit(user_limit: Option<u32>) -> u32 {
        user_limit.unwrap_or(512)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_compute_default_pid_limit() {
        assert_eq!(ProcessLimitController::compute_default_pid_limit(None), 512);
        assert_eq!(
            ProcessLimitController::compute_default_pid_limit(Some(128)),
            128
        );
    }

    #[test]
    fn test_apply_cgroup_pid_limit() {
        let temp = TempDir::new().unwrap();
        let res = ProcessLimitController::apply_cgroup_pid_limit(temp.path(), 256);
        assert!(res.is_ok());
    }
}
