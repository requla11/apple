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
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if std::fs::hard_link(src, dst).is_err() {
                std::fs::copy(src, dst)?;
            }
            return Ok(());
        }

        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let entry_path = entry.path();
            let dest_path = dst.join(entry.file_name());
            if entry_path.is_dir() {
                Self::mirror_hardlink_tree(&entry_path, &dest_path)?;
            } else {
                if let Some(parent) = dest_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                if std::fs::hard_link(&entry_path, &dest_path).is_err() {
                    std::fs::copy(&entry_path, &dest_path)?;
                }
            }
        }
        Ok(())
    }

    fn link_or_copy_readonly(&self, src: &Path, dst: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if src.is_file() {
            if std::fs::hard_link(src, dst).is_err() {
                std::fs::copy(src, dst)?;
            }
        } else if src.is_dir() {
            Self::mirror_hardlink_tree(src, dst)?;
        }
        Ok(())
    }
}
