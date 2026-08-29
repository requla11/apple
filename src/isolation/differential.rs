use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct DifferentialArtifactSynchronizer;

impl DifferentialArtifactSynchronizer {
    pub fn take_snapshot(jail_root: &Path) -> Result<HashMap<PathBuf, (u64, u64)>, std::io::Error> {
        let mut snapshot = HashMap::new();
        if !jail_root.exists() {
            return Ok(snapshot);
        }
        Self::collect_metadata(jail_root, jail_root, &mut snapshot)?;
        Ok(snapshot)
    }

    fn collect_metadata(
        root: &Path,
        current: &Path,
        out: &mut HashMap<PathBuf, (u64, u64)>,
    ) -> Result<(), std::io::Error> {
        for entry in std::fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                Self::collect_metadata(root, &path, out)?;
                continue;
            }
            if !path.is_file() {
                continue;
            }
            let Ok(rel) = path.strip_prefix(root) else {
                continue;
            };
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let size = meta.len();
            out.insert(rel.to_path_buf(), (mtime, size));
        }
        Ok(())
    }

    pub fn extract_modified_artifacts(
        jail_root: &Path,
        workspace_target: &Path,
        initial_snapshot: &HashMap<PathBuf, (u64, u64)>,
    ) -> Result<Vec<PathBuf>, std::io::Error> {
        let mut synchronized = Vec::new();
        let current_snapshot = Self::take_snapshot(jail_root)?;

        for (rel_path, (curr_mtime, curr_size)) in current_snapshot {
            let is_new_or_modified = match initial_snapshot.get(&rel_path) {
                None => true,
                Some((init_mtime, init_size)) => {
                    curr_mtime > *init_mtime || curr_size != *init_size
                }
            };

            if is_new_or_modified {
                let rel_str = rel_path.to_string_lossy();
                if rel_str.starts_with("tmp") || rel_str.starts_with(".apple-scratch") {
                    continue;
                }

                let src = jail_root.join(&rel_path);
                let dst = workspace_target.join(&rel_path);

                if let Some(parent) = dst.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                std::fs::copy(&src, &dst)?;
                synchronized.push(rel_path);
            }
        }

        Ok(synchronized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_differential_artifact_sync_identifies_new_files() {
        let temp = TempDir::new().unwrap();
        let jail = temp.path().join("jail");
        let workspace = temp.path().join("workspace");

        std::fs::create_dir_all(&jail).unwrap();
        std::fs::create_dir_all(&workspace).unwrap();

        std::fs::write(jail.join("existing.txt"), b"unchanged").unwrap();
        let snapshot = DifferentialArtifactSynchronizer::take_snapshot(&jail).unwrap();

        std::fs::write(jail.join("artifact.bin"), b"binary output").unwrap();
        let synced = DifferentialArtifactSynchronizer::extract_modified_artifacts(
            &jail, &workspace, &snapshot,
        )
        .unwrap();

        assert_eq!(synced.len(), 1);
        assert_eq!(synced[0], PathBuf::from("artifact.bin"));
        assert!(workspace.join("artifact.bin").exists());
        assert_eq!(
            std::fs::read(workspace.join("artifact.bin")).unwrap(),
            b"binary output"
        );
    }
}
