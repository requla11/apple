use crate::protocol::{MountKind, MountRule};
use std::path::{Path, PathBuf};

pub struct HermeticFilesystemManager {
    scratch_root: PathBuf,
}

impl HermeticFilesystemManager {
    pub fn new(scratch_root: impl Into<PathBuf>) -> Self {
        Self {
            scratch_root: scratch_root.into(),
        }
    }

    pub fn prepare_workspace_jail(
        &self,
        task_id: &str,
        rules: &[MountRule],
    ) -> Result<PathBuf, std::io::Error> {
        let jail_dir = self.scratch_root.join(format!("jail_{task_id}"));
        if jail_dir.exists() {
            std::fs::remove_dir_all(&jail_dir)?;
        }
        std::fs::create_dir_all(&jail_dir)?;

        for rule in rules {
            match rule.kind {
                MountKind::ReadOnly => {
                    self.link_or_copy_readonly(&rule.source, &jail_dir.join(&rule.target))?;
                }
                MountKind::ReadWrite | MountKind::Tmpfs | MountKind::Overlay => {
                    let dest = jail_dir.join(&rule.target);
                    std::fs::create_dir_all(&dest)?;
                }
            }
        }
        Ok(jail_dir)
    }

    pub fn cleanup_jail(&self, task_id: &str) -> Result<(), std::io::Error> {
        let jail_dir = self.scratch_root.join(format!("jail_{task_id}"));
        if jail_dir.exists() {
            std::fs::remove_dir_all(&jail_dir)?;
        }
        Ok(())
    }

    pub fn mirror_hardlink_tree(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
        if !src.exists() {
            return Ok(());
        }
        if src.is_file() {
            return crate::isolation::cow::CowCloner::clone_file(src, dst);
        }

        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.is_dir() && entry.file_name() == ".apple-scratch" {
                continue;
            }
            let dest_path = dst.join(entry.file_name());
            if entry_path.is_dir() {
                Self::mirror_hardlink_tree(&entry_path, &dest_path)?;
            } else {
                crate::isolation::cow::CowCloner::clone_file(&entry_path, &dest_path)?;
            }
        }
        Ok(())
    }

    fn link_or_copy_readonly(&self, src: &Path, dst: &Path) -> Result<(), std::io::Error> {
        if src.is_dir() {
            Self::mirror_hardlink_tree(src, dst)
        } else if src.is_file() {
            crate::isolation::cow::CowCloner::clone_file(src, dst)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{MountKind, MountRule};

    #[test]
    fn test_hermetic_filesystem_manager_lifecycle() {
        let temp_dir = std::env::temp_dir().join(format!("apple_fs_test_{}", std::process::id()));
        let manager = HermeticFilesystemManager::new(&temp_dir);

        let src_file = temp_dir.join("src.txt");
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(&src_file, "hello hermetic world").unwrap();

        let rules = vec![
            MountRule {
                source: src_file.clone(),
                target: PathBuf::from("input.txt"),
                kind: MountKind::ReadOnly,
            },
            MountRule {
                source: PathBuf::from("/tmp"),
                target: PathBuf::from("output"),
                kind: MountKind::ReadWrite,
            },
        ];

        let jail = manager
            .prepare_workspace_jail("test_task_1", &rules)
            .unwrap();
        assert!(jail.exists());
        assert!(jail.join("input.txt").exists());
        assert!(jail.join("output").exists());

        let content = std::fs::read_to_string(jail.join("input.txt")).unwrap();
        assert_eq!(content, "hello hermetic world");

        manager.cleanup_jail("test_task_1").unwrap();
        assert!(!jail.exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
