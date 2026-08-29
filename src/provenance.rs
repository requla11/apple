use crate::protocol::{ExecutionRequest, ExecutionResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceDescriptor {
    pub name: String,
    pub digest: HashMap<String, String>,
}

impl ResourceDescriptor {
    pub fn from_file(path: &Path) -> Result<Self, std::io::Error> {
        let content = std::fs::read(path)?;
        let mut hasher = blake3::Hasher::new();
        hasher.update(&content);
        let hash = hasher.finalize().to_hex().to_string();

        let mut digest = HashMap::new();
        digest.insert("blake3".to_string(), hash);

        Ok(Self {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            digest,
        })
    }

    pub fn from_bytes(name: impl Into<String>, bytes: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(bytes);
        let hash = hasher.finalize().to_hex().to_string();

        let mut digest = HashMap::new();
        digest.insert("blake3".to_string(), hash);

        Self {
            name: name.into(),
            digest,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuilderInfo {
    pub id: String,
    pub version: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildMetadata {
    pub invocation_id: String,
    pub execution_duration_ms: u64,
    pub hermetic_guarantee: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildDefinition {
    pub build_type: String,
    pub external_parameters: serde_json::Value,
    pub internal_parameters: serde_json::Value,
    pub resolved_dependencies: Vec<ResourceDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunDetails {
    pub builder: BuilderInfo,
    pub metadata: BuildMetadata,
    pub byproducts: Vec<ResourceDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlsaPredicate {
    pub build_definition: BuildDefinition,
    pub run_details: RunDetails,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlsaStatement {
    #[serde(rename = "_type")]
    pub statement_type: String,
    pub subject: Vec<ResourceDescriptor>,
    pub predicate_type: String,
    pub predicate: SlsaPredicate,
}

pub struct SlsaProvenanceGenerator;

impl SlsaProvenanceGenerator {
    pub fn generate(
        request: &ExecutionRequest,
        result: &ExecutionResult,
        output_paths: &[PathBuf],
    ) -> Result<SlsaStatement, std::io::Error> {
        let mut subject = Vec::new();
        for out in output_paths {
            if out.exists() && out.is_file() {
                if let Ok(rd) = ResourceDescriptor::from_file(out) {
                    subject.push(rd);
                }
            }
        }

        let mut resolved_dependencies = Vec::new();
        for input in &request.profile.declared_inputs {
            if input.exists() && input.is_file() {
                if let Ok(rd) = ResourceDescriptor::from_file(input) {
                    resolved_dependencies.push(rd);
                }
            }
        }

        let mut builder_version = HashMap::new();
        builder_version.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());

        let statement = SlsaStatement {
            statement_type: "https://in-toto.io/Statement/v1".to_string(),
            subject,
            predicate_type: "https://slsa.dev/provenance/v1".to_string(),
            predicate: SlsaPredicate {
                build_definition: BuildDefinition {
                    build_type: "https://github.com/requla11/apple/build-definition/v1".to_string(),
                    external_parameters: serde_json::json!({
                        "argv": request.argv,
                        "working_dir": request.working_dir.to_string_lossy(),
                    }),
                    internal_parameters: serde_json::json!({
                        "profile_name": request.profile.name,
                        "isolation_level": format!("{:?}", request.profile.level),
                        "allow_network": request.profile.allow_network,
                    }),
                    resolved_dependencies,
                },
                run_details: RunDetails {
                    builder: BuilderInfo {
                        id: "https://github.com/requla11/apple".to_string(),
                        version: builder_version,
                    },
                    metadata: BuildMetadata {
                        invocation_id: request.task_id.clone(),
                        execution_duration_ms: result.execution_duration_ms,
                        hermetic_guarantee: result.hermetic_guarantee,
                    },
                    byproducts: Vec::new(),
                },
            },
        };

        Ok(statement)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{IsolationLevel, SandboxProfile};
    use tempfile::TempDir;

    #[test]
    fn test_slsa_provenance_generation() {
        let temp = TempDir::new().unwrap();
        let artifact = temp.path().join("libsample.a");
        std::fs::write(&artifact, b"fake binary artifact").unwrap();

        let input_file = temp.path().join("main.c");
        std::fs::write(&input_file, b"int main() { return 0; }").unwrap();

        let req = ExecutionRequest {
            task_id: "task_slsa_01".to_string(),
            working_dir: temp.path().to_path_buf(),
            argv: vec![
                "gcc".to_string(),
                "main.c".to_string(),
                "-o".to_string(),
                "libsample.a".to_string(),
            ],
            env: HashMap::new(),
            profile: SandboxProfile {
                name: "c-hermetic".to_string(),
                level: IsolationLevel::StrictFilesystem,
                allow_network: false,
                memory_limit_mb: Some(1024),
                cpu_affinity_mask: None,
                timeout_seconds: Some(60),
                mount_rules: Vec::new(),
                whitelisted_env: Vec::new(),
                seccomp_filter: true,
                appcontainer: false,
                declared_inputs: vec![input_file.clone()],
            },
            keep_jail: false,
        };

        let res = ExecutionResult {
            task_id: "task_slsa_01".to_string(),
            exit_code: 0,
            stdout: Vec::new(),
            stderr: Vec::new(),
            execution_duration_ms: 42,
            peak_memory_bytes: 1048576,
            violations: Vec::new(),
            hermetic_guarantee: true,
        };

        let stmt = SlsaProvenanceGenerator::generate(&req, &res, &[artifact.clone()]).unwrap();
        assert_eq!(stmt.statement_type, "https://in-toto.io/Statement/v1");
        assert_eq!(stmt.predicate_type, "https://slsa.dev/provenance/v1");
        assert_eq!(stmt.subject.len(), 1);
        assert_eq!(stmt.subject[0].name, "libsample.a");
        assert!(stmt.subject[0].digest.contains_key("blake3"));
        assert_eq!(
            stmt.predicate.build_definition.resolved_dependencies.len(),
            1
        );
    }
}
