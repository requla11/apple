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
