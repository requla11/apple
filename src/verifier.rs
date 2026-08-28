use crate::daemon::AppleDaemonServer;
use crate::protocol::ExecutionRequest;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub task_id: String,
    pub artifact_path: String,
    pub is_deterministic: bool,
    pub pass1_hash: String,
    pub pass2_hash: String,
    pub pass1_duration_ms: u64,
    pub pass2_duration_ms: u64,
}

pub struct DeterminismVerifier {
    daemon: AppleDaemonServer,
}

impl DeterminismVerifier {
    pub fn new(scratch_dir: PathBuf) -> Self {
        Self {
            daemon: AppleDaemonServer::new(scratch_dir),
        }
    }

    pub async fn verify_reproducible(
        &self,
        mut request: ExecutionRequest,
        artifact_rel_path: &Path,
    ) -> Result<VerificationReport, anyhow::Error> {
        let task_base = request.task_id.clone();

        request.task_id = format!("{task_base}_pass1");
        let res1 = self.daemon.execute_task(request.clone()).await;
        if res1.exit_code != 0 {
            return Err(anyhow::anyhow!(
                "pass 1 execution failed with exit code {}: {}",
                res1.exit_code,
                String::from_utf8_lossy(&res1.stderr)
            ));
        }

        let pass1_artifact = request.working_dir.join(artifact_rel_path);
        let hash1 = Self::hash_file_if_exists(&pass1_artifact)?;

        request.task_id = format!("{task_base}_pass2");
        request
            .env
            .insert("SOURCE_DATE_EPOCH".to_string(), "1700000000".to_string());
        request.env.insert("TZ".to_string(), "UTC".to_string());
        request.env.insert("LC_ALL".to_string(), "C".to_string());

        let res2 = self.daemon.execute_task(request.clone()).await;
        if res2.exit_code != 0 {
            return Err(anyhow::anyhow!(
                "pass 2 execution failed with exit code {}: {}",
                res2.exit_code,
                String::from_utf8_lossy(&res2.stderr)
            ));
        }

        let pass2_artifact = request.working_dir.join(artifact_rel_path);
        let hash2 = Self::hash_file_if_exists(&pass2_artifact)?;

        let is_deterministic = hash1 == hash2 && !hash1.is_empty();

        Ok(VerificationReport {
            task_id: task_base,
            artifact_path: artifact_rel_path.display().to_string(),
            is_deterministic,
            pass1_hash: hash1,
            pass2_hash: hash2,
            pass1_duration_ms: res1.execution_duration_ms,
            pass2_duration_ms: res2.execution_duration_ms,
        })
    }

    fn hash_file_if_exists(path: &Path) -> Result<String, std::io::Error> {
        if !path.exists() {
            return Ok(String::new());
        }
        let data = std::fs::read(path)?;
        let hash = blake3::hash(&data);
        Ok(hash.to_hex().to_string())
    }
}
