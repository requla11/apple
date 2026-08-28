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

    fn link_or_copy_readonly(&self, src: &Path, dst: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if src.is_file() {
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(src, dst)?;
            }
            #[cfg(windows)]
            {
                std::fs::copy(src, dst)?;
            }
        }
        Ok(())
    }
}
