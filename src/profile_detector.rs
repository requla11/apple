use crate::protocol::{IsolationLevel, MountKind, MountRule, SandboxProfile};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageKind {
    Rust,
    Cpp,
    Go,
    NodeJs,
    Python,
    Generic,
}

pub struct ProfileDetector;

impl ProfileDetector {
    pub fn detect_language(root_dir: &Path) -> LanguageKind {
        if root_dir.join("Cargo.toml").exists() {
            LanguageKind::Rust
        } else if root_dir.join("go.mod").exists() {
            LanguageKind::Go
        } else if root_dir.join("package.json").exists() {
            LanguageKind::NodeJs
        } else if root_dir.join("CMakeLists.txt").exists() || root_dir.join("Makefile").exists() {
            LanguageKind::Cpp
        } else if root_dir.join("pyproject.toml").exists()
            || root_dir.join("requirements.txt").exists()
        {
            LanguageKind::Python
        } else {
            LanguageKind::Generic
        }
    }

    pub fn auto_generate_profile(root_dir: &Path) -> SandboxProfile {
        let lang = Self::detect_language(root_dir);
        let mut profile = SandboxProfile::default();
        profile.level = IsolationLevel::FullHermetic;

        match lang {
            LanguageKind::Rust => {
                profile.name = "auto-rust-hermetic".to_string();
                profile.allow_network = false;
                profile.whitelisted_env.extend([
                    "RUSTFLAGS".to_string(),
                    "RUSTC".to_string(),
                    "CARGO_HOME".to_string(),
                    "CARGO_TARGET_DIR".to_string(),
                ]);
                profile.mount_rules.push(MountRule {
                    source: root_dir.to_path_buf(),
                    target: PathBuf::from("workspace"),
                    kind: MountKind::ReadOnly,
                });
                profile.mount_rules.push(MountRule {
                    source: root_dir.join("target"),
                    target: PathBuf::from("workspace/target"),
                    kind: MountKind::ReadWrite,
                });
            }
            LanguageKind::Cpp => {
                profile.name = "auto-cpp-hermetic".to_string();
                profile.allow_network = false;
                profile.whitelisted_env.extend([
                    "CC".to_string(),
                    "CXX".to_string(),
                    "CFLAGS".to_string(),
                    "CXXFLAGS".to_string(),
                    "LDFLAGS".to_string(),
                ]);
                profile.mount_rules.push(MountRule {
                    source: root_dir.to_path_buf(),
                    target: PathBuf::from("workspace"),
                    kind: MountKind::ReadOnly,
                });
            }
            LanguageKind::Go => {
                profile.name = "auto-go-hermetic".to_string();
                profile.allow_network = false;
                profile.whitelisted_env.extend([
                    "GOROOT".to_string(),
                    "GOPATH".to_string(),
                    "GOPROXY".to_string(),
                    "GOCACHE".to_string(),
                ]);
                profile.mount_rules.push(MountRule {
                    source: root_dir.to_path_buf(),
                    target: PathBuf::from("workspace"),
                    kind: MountKind::ReadOnly,
                });
            }
            LanguageKind::NodeJs => {
                profile.name = "auto-node-hermetic".to_string();
                profile.allow_network = false;
                profile
                    .whitelisted_env
                    .extend(["NODE_ENV".to_string(), "NPM_CONFIG_OFFLINE".to_string()]);
                profile.mount_rules.push(MountRule {
                    source: root_dir.to_path_buf(),
                    target: PathBuf::from("workspace"),
                    kind: MountKind::ReadOnly,
                });
            }
            LanguageKind::Python => {
                profile.name = "auto-python-hermetic".to_string();
                profile.allow_network = false;
                profile
                    .whitelisted_env
                    .extend(["PYTHONPATH".to_string(), "PIP_NO_INDEX".to_string()]);
                profile.mount_rules.push(MountRule {
                    source: root_dir.to_path_buf(),
                    target: PathBuf::from("workspace"),
                    kind: MountKind::ReadOnly,
                });
            }
            LanguageKind::Generic => {
                profile.name = "auto-generic-hermetic".to_string();
                profile.allow_network = false;
                profile.mount_rules.push(MountRule {
                    source: root_dir.to_path_buf(),
                    target: PathBuf::from("workspace"),
                    kind: MountKind::ReadOnly,
                });
            }
        }

        profile
    }
}
