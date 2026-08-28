use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolationLevel {
    Off,
    ProcessOnly,
    StrictFilesystem,
    FullHermetic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MountKind {
    ReadOnly,
    ReadWrite,
    Tmpfs,
    Overlay,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountRule {
    pub source: PathBuf,
    pub target: PathBuf,
    pub kind: MountKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxProfile {
    pub name: String,
    pub level: IsolationLevel,
    pub allow_network: bool,
    pub memory_limit_mb: Option<u64>,
    pub cpu_affinity_mask: Option<u64>,
    pub timeout_seconds: Option<u64>,
    pub mount_rules: Vec<MountRule>,
    pub whitelisted_env: Vec<String>,
}

impl Default for SandboxProfile {
    fn default() -> Self {
        Self {
            name: "default-hermetic".to_string(),
            level: IsolationLevel::StrictFilesystem,
            allow_network: false,
            memory_limit_mb: Some(4096),
            cpu_affinity_mask: None,
            timeout_seconds: Some(300),
            mount_rules: Vec::new(),
            whitelisted_env: vec![
                "PATH".to_string(),
                "TERM".to_string(),
                "LANG".to_string(),
                "RUSTFLAGS".to_string(),
                "CFLAGS".to_string(),
                "CXXFLAGS".to_string(),
                "GOOS".to_string(),
                "GOARCH".to_string(),
                "NODE_ENV".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub task_id: String,
    pub working_dir: PathBuf,
    pub argv: Vec<String>,
    pub env: HashMap<String, String>,
    pub profile: SandboxProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViolationRecord {
    pub target_path: Option<PathBuf>,
    pub operation: String,
    pub description: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub task_id: String,
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub execution_duration_ms: u64,
    pub peak_memory_bytes: u64,
    pub violations: Vec<ViolationRecord>,
    pub hermetic_guarantee: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DaemonMessage {
    Ping,
    Pong {
        version: String,
        active_sandboxes: usize,
    },
    Execute(ExecutionRequest),
    Result(ExecutionResult),
    Cancel {
        task_id: String,
    },
    Shutdown,
}
