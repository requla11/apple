use crate::protocol::ViolationRecord;
use std::path::{Path, PathBuf};

pub struct LiveIoInterceptor {
    allowed_roots: Vec<PathBuf>,
    declared_inputs: Vec<PathBuf>,
    secret_patterns: Vec<&'static str>,
}

impl LiveIoInterceptor {
    pub fn new(allowed_roots: Vec<PathBuf>, declared_inputs: Vec<PathBuf>) -> Self {
        Self {
            allowed_roots,
            declared_inputs,
            secret_patterns: vec![
                ".env",
                ".env.local",
                ".env.production",
                ".env.staging",
                "id_rsa",
                "id_ed25519",
                "id_ecdsa",
                "known_hosts",
                ".aws/credentials",
                ".aws/config",
                "/etc/shadow",
                "/etc/sudoers",
                "/root",
            ],
        }
    }

    pub fn inspect_path_access(&self, target: &Path, is_write: bool) -> Option<ViolationRecord> {
        let path_str = target.to_string_lossy();

        for secret in &self.secret_patterns {
            if path_str.contains(secret) {
                return Some(ViolationRecord {
                    target_path: Some(target.to_path_buf()),
                    operation: if is_write { "WRITE" } else { "READ" }.to_string(),
                    description: format!(
                        "Critical secret access violation: target contains prohibited secret pattern `{secret}`"
                    ),
                    timestamp_ms: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                });
            }
        }

        let inside_root = self.allowed_roots.iter().any(|r| target.starts_with(r));
        if !inside_root {
            return Some(ViolationRecord {
                target_path: Some(target.to_path_buf()),
                operation: if is_write { "WRITE" } else { "READ" }.to_string(),
                description: format!(
                    "Undeclared filesystem access outside allowed mount roots: {}",
                    target.display()
                ),
                timestamp_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            });
        }

        if !is_write && !self.declared_inputs.is_empty() {
            let is_declared = self.declared_inputs.iter().any(|input| {
                target == input || target.starts_with(input) || input.starts_with(target)
            });

            if !is_declared && !path_str.contains("/tmp") && !path_str.contains("/target") {
                return Some(ViolationRecord {
                    target_path: Some(target.to_path_buf()),
                    operation: "READ".to_string(),
                    description: format!(
                        "Undeclared DAG input header or file probed: {}",
                        target.display()
                    ),
                    timestamp_ms: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                });
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_file_probes_trigger_violation() {
        let roots = vec![PathBuf::from("/workspace/jail")];
        let declared = vec![PathBuf::from("/workspace/jail/src/main.rs")];
        let interceptor = LiveIoInterceptor::new(roots, declared);

        let v1 = interceptor.inspect_path_access(Path::new("/workspace/jail/.env"), false);
        assert!(v1.is_some());
        assert!(v1.unwrap().description.contains("prohibited secret"));

        let v2 = interceptor.inspect_path_access(Path::new("/home/user/.ssh/id_rsa"), false);
        assert!(v2.is_some());
    }

    #[test]
    fn test_undeclared_dag_header_trigger_violation() {
        let roots = vec![PathBuf::from("/workspace/jail")];
        let declared = vec![PathBuf::from("/workspace/jail/include/math.h")];
        let interceptor = LiveIoInterceptor::new(roots, declared);

        let allowed =
            interceptor.inspect_path_access(Path::new("/workspace/jail/include/math.h"), false);
        assert!(allowed.is_none());

        let undeclared =
            interceptor.inspect_path_access(Path::new("/workspace/jail/secret/internal.h"), false);
        assert!(undeclared.is_some());
        assert!(
            undeclared
                .unwrap()
                .description
                .contains("Undeclared DAG input")
        );
    }
}
