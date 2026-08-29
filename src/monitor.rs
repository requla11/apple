use crate::protocol::ViolationRecord;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct SandboxMonitor {
    allowed_roots: Vec<PathBuf>,
    violation_count: AtomicUsize,
}

impl SandboxMonitor {
    pub fn new(allowed_roots: Vec<PathBuf>) -> Self {
        Self {
            allowed_roots,
            violation_count: AtomicUsize::new(0),
        }
    }

    pub fn inspect_access(&self, target: &Path, is_write: bool) -> Option<ViolationRecord> {
        let is_allowed = self
            .allowed_roots
            .iter()
            .any(|root| target.starts_with(root));
        if !is_allowed {
            self.violation_count.fetch_add(1, Ordering::Relaxed);
            return Some(ViolationRecord {
                target_path: Some(target.to_path_buf()),
                operation: if is_write { "WRITE" } else { "READ" }.to_string(),
                description: format!(
                    "Unauthorized filesystem access outside hermetic jail: {}",
                    target.display()
                ),
                timestamp_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            });
        }
        None
    }

    pub fn violation_count(&self) -> usize {
        self.violation_count.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_monitor_detects_violations() {
        let allowed = vec![PathBuf::from("/workspace/jail"), PathBuf::from("/tmp")];
        let monitor = SandboxMonitor::new(allowed);

        assert!(
            monitor
                .inspect_access(Path::new("/workspace/jail/src/main.rs"), false)
                .is_none()
        );
        assert!(
            monitor
                .inspect_access(Path::new("/tmp/scratch.tmp"), true)
                .is_none()
        );

        let violation = monitor.inspect_access(Path::new("/etc/passwd"), false);
        assert!(violation.is_some());
        assert_eq!(monitor.violation_count(), 1);

        let v = violation.unwrap();
        assert_eq!(v.operation, "READ");
        assert_eq!(v.target_path, Some(PathBuf::from("/etc/passwd")));
    }
}
