use crate::isolation::{
    HermeticEnvironmentSanitizer, HermeticFilesystemManager, NetworkIsolationController,
    ProcessIsolationRunner,
};
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
    scratch_dir: PathBuf,
}

impl DeterminismVerifier {
    pub fn new(scratch_dir: PathBuf) -> Self {
        Self { scratch_dir }
    }

    pub async fn verify_reproducible(
        &self,
        request: ExecutionRequest,
        artifact_rel_path: &Path,
    ) -> Result<VerificationReport, anyhow::Error> {
        let task_base = request.task_id.clone();

        let res1 = self.run_pass(&request, "pass1", false).await?;
        if res1.exit_code != 0 {
            self.cleanup_pass(&task_base, "pass1");
            return Err(anyhow::anyhow!(
                "pass 1 execution failed with exit code {}: {}",
                res1.exit_code,
                String::from_utf8_lossy(&res1.stderr)
            ));
        }
        let hash1 = self.hash_jail_artifact(&format!("{task_base}_pass1"), artifact_rel_path);
        self.cleanup_pass(&task_base, "pass1");
        let hash1 = hash1?;

        let res2 = self.run_pass(&request, "pass2", true).await?;
        if res2.exit_code != 0 {
            self.cleanup_pass(&task_base, "pass2");
            return Err(anyhow::anyhow!(
                "pass 2 execution failed with exit code {}: {}",
                res2.exit_code,
                String::from_utf8_lossy(&res2.stderr)
            ));
        }
        let hash2 = self.hash_jail_artifact(&format!("{task_base}_pass2"), artifact_rel_path);
        self.cleanup_pass(&task_base, "pass2");
        let hash2 = hash2?;

        let is_deterministic = !hash1.is_empty() && hash1 == hash2;

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

    async fn run_pass(
        &self,
        base: &ExecutionRequest,
        suffix: &str,
        perturb_env: bool,
    ) -> Result<crate::protocol::ExecutionResult, anyhow::Error> {
        let fs_mgr = HermeticFilesystemManager::new(&self.scratch_dir);

        let mut request = base.clone();
        request.task_id = format!("{}_{}", base.task_id, suffix);

        let jail = fs_mgr.prepare_workspace_jail(&request.task_id, &request.profile.mount_rules)?;
        let tmp_dir = jail.join("tmp");
        std::fs::create_dir_all(&tmp_dir)?;

        let mut env = HermeticEnvironmentSanitizer::sanitize(
            &request.env,
            &request.profile.whitelisted_env,
            Some(&tmp_dir),
        );
        NetworkIsolationController::apply_network_policy(&mut env, request.profile.allow_network);

        if perturb_env {
            env.insert("SOURCE_DATE_EPOCH".to_string(), "1700000000".to_string());
            env.insert("TZ".to_string(), "UTC".to_string());
            env.insert("LC_ALL".to_string(), "C".to_string());
        }
        request.env = env;
        request.working_dir = jail;

        ProcessIsolationRunner::run_sandboxed(request).await
    }

    fn hash_jail_artifact(
        &self,
        task_id_with_suffix: &str,
        artifact_rel_path: &Path,
    ) -> Result<String, anyhow::Error> {
        let jail_artifact = self
            .scratch_dir
            .join(format!("jail_{task_id_with_suffix}"))
            .join(artifact_rel_path);
        if !jail_artifact.exists() {
            anyhow::bail!(
                "artifact `{}` was not produced inside the sandbox jail (looked for `{}`)",
                artifact_rel_path.display(),
                jail_artifact.display()
            );
        }
        let data = std::fs::read(&jail_artifact)?;
        Ok(blake3::hash(&data).to_hex().to_string())
    }

    fn cleanup_pass(&self, base_task_id: &str, suffix: &str) {
        let fs_mgr = HermeticFilesystemManager::new(&self.scratch_dir);
        let _ = fs_mgr.cleanup_jail(&format!("{base_task_id}_{suffix}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_report_creation() {
        let report = VerificationReport {
            task_id: "task_123".to_string(),
            artifact_path: "target/release/app".to_string(),
            is_deterministic: true,
            pass1_hash: "abc".to_string(),
            pass2_hash: "abc".to_string(),
            pass1_duration_ms: 100,
            pass2_duration_ms: 95,
        };

        assert!(report.is_deterministic);
        assert_eq!(report.pass1_hash, report.pass2_hash);
    }
}
