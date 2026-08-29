use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpdxChecksum {
    pub algorithm: String,
    pub checksum_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpdxPackage {
    pub name: String,
    #[serde(rename = "SPDXID")]
    pub spdx_id: String,
    pub download_location: String,
    pub files_analyzed: bool,
    pub checksums: Vec<SpdxChecksum>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpdxCreationInfo {
    pub creators: Vec<String>,
    pub created: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpdxDocument {
    pub spdx_version: String,
    pub data_license: String,
    #[serde(rename = "SPDXID")]
    pub spdx_id: String,
    pub name: String,
    pub document_namespace: String,
    pub creation_info: SpdxCreationInfo,
    pub packages: Vec<SpdxPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CycloneDxHash {
    pub alg: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CycloneDxComponent {
    #[serde(rename = "type")]
    pub component_type: String,
    pub name: String,
    pub version: String,
    pub hashes: Vec<CycloneDxHash>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CycloneDxTool {
    pub vendor: String,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CycloneDxMetadata {
    pub timestamp: String,
    pub tools: Vec<CycloneDxTool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CycloneDxBom {
    pub bom_format: String,
    pub spec_version: String,
    pub serial_number: String,
    pub version: u32,
    pub metadata: CycloneDxMetadata,
    pub components: Vec<CycloneDxComponent>,
}

pub struct SbomGenerator;

impl SbomGenerator {
    pub fn compute_blake3_hex(path: &Path) -> Result<String, std::io::Error> {
        let content = std::fs::read(path)?;
        let mut hasher = blake3::Hasher::new();
        hasher.update(&content);
        Ok(hasher.finalize().to_hex().to_string())
    }

    pub fn generate_spdx(
        task_id: &str,
        artifacts: &[PathBuf],
    ) -> Result<SpdxDocument, std::io::Error> {
        let mut packages = Vec::new();

        for (idx, artifact) in artifacts.iter().enumerate() {
            if artifact.exists() && artifact.is_file() {
                let hash = Self::compute_blake3_hex(artifact)?;
                let file_name = artifact
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                packages.push(SpdxPackage {
                    name: file_name,
                    spdx_id: format!("SPDXRef-Package-{idx}"),
                    download_location: "NOASSERTION".to_string(),
                    files_analyzed: true,
                    checksums: vec![SpdxChecksum {
                        algorithm: "BLAKE3".to_string(),
                        checksum_value: hash,
                    }],
                });
            }
        }

        Ok(SpdxDocument {
            spdx_version: "SPDX-2.3".to_string(),
            data_license: "CC0-1.0".to_string(),
            spdx_id: "SPDXRef-DOCUMENT".to_string(),
            name: format!("apple-sbom-{task_id}"),
            document_namespace: format!("https://github.com/requla11/apple/spdx/{task_id}"),
            creation_info: SpdxCreationInfo {
                creators: vec![format!("Tool: apple-{}", env!("CARGO_PKG_VERSION"))],
                created: "2026-08-29T20:00:00Z".to_string(),
            },
            packages,
        })
    }

    pub fn generate_cyclonedx(
        task_id: &str,
        artifacts: &[PathBuf],
    ) -> Result<CycloneDxBom, std::io::Error> {
        let mut components = Vec::new();

        for artifact in artifacts {
            if artifact.exists() && artifact.is_file() {
                let hash = Self::compute_blake3_hex(artifact)?;
                let file_name = artifact
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                components.push(CycloneDxComponent {
                    component_type: "file".to_string(),
                    name: file_name,
                    version: "1.0.0".to_string(),
                    hashes: vec![CycloneDxHash {
                        alg: "BLAKE3".to_string(),
                        content: hash,
                    }],
                });
            }
        }

        Ok(CycloneDxBom {
            bom_format: "CycloneDX".to_string(),
            spec_version: "1.5".to_string(),
            serial_number: format!("urn:uuid:apple-sbom-{task_id}"),
            version: 1,
            metadata: CycloneDxMetadata {
                timestamp: "2026-08-29T20:00:00Z".to_string(),
                tools: vec![CycloneDxTool {
                    vendor: "requla11".to_string(),
                    name: "apple".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                }],
            },
            components,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sbom_spdx_and_cyclonedx_generation() {
        let temp = TempDir::new().unwrap();
        let file1 = temp.path().join("app.exe");
        let file2 = temp.path().join("libcore.dylib");

        std::fs::write(&file1, b"PE executable header").unwrap();
        std::fs::write(&file2, b"Mach-O dylib header").unwrap();

        let artifacts = vec![file1, file2];

        let spdx = SbomGenerator::generate_spdx("task_999", &artifacts).unwrap();
        assert_eq!(spdx.spdx_version, "SPDX-2.3");
        assert_eq!(spdx.packages.len(), 2);
        assert_eq!(spdx.packages[0].name, "app.exe");
        assert_eq!(spdx.packages[0].checksums[0].algorithm, "BLAKE3");

        let cdx = SbomGenerator::generate_cyclonedx("task_999", &artifacts).unwrap();
        assert_eq!(cdx.bom_format, "CycloneDX");
        assert_eq!(cdx.spec_version, "1.5");
        assert_eq!(cdx.components.len(), 2);
        assert_eq!(cdx.components[1].name, "libcore.dylib");
        assert_eq!(cdx.components[1].hashes[0].alg, "BLAKE3");
    }
}
