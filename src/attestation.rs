use crate::provenance::SlsaStatement;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttestationEnvelope {
    pub payload_type: String,
    pub payload: String,
    pub signature: String,
    pub key_id: String,
}

pub struct AttestationSigner;

impl AttestationSigner {
    pub fn sign_statement(
        statement: &SlsaStatement,
        secret_key: &[u8; 32],
        key_id: &str,
    ) -> Result<AttestationEnvelope, serde_json::Error> {
        let payload = serde_json::to_string(statement)?;
        let mac = blake3::keyed_hash(secret_key, payload.as_bytes());

        Ok(AttestationEnvelope {
            payload_type: "application/vnd.in-toto+json".to_string(),
            payload,
            signature: mac.to_hex().to_string(),
            key_id: key_id.to_string(),
        })
    }

    pub fn verify_envelope(
        envelope: &AttestationEnvelope,
        secret_key: &[u8; 32],
    ) -> Result<bool, serde_json::Error> {
        let expected_mac = blake3::keyed_hash(secret_key, envelope.payload.as_bytes());
        let expected_hex = expected_mac.to_hex().to_string();

        if envelope.signature != expected_hex {
            return Ok(false);
        }

        let _: SlsaStatement = serde_json::from_str(&envelope.payload)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::*;
    use std::collections::HashMap;

    #[test]
    fn test_attestation_signing_and_verification() {
        let key = [42u8; 32];
        let statement = SlsaStatement {
            statement_type: "https://in-toto.io/Statement/v1".to_string(),
            subject: vec![ResourceDescriptor::from_bytes(
                "test.bin",
                b"hello attestation",
            )],
            predicate_type: "https://slsa.dev/provenance/v1".to_string(),
            predicate: SlsaPredicate {
                build_definition: BuildDefinition {
                    build_type: "https://github.com/requla11/apple/build-definition/v1".to_string(),
                    external_parameters: serde_json::json!({}),
                    internal_parameters: serde_json::json!({}),
                    resolved_dependencies: Vec::new(),
                },
                run_details: RunDetails {
                    builder: BuilderInfo {
                        id: "https://github.com/requla11/apple".to_string(),
                        version: HashMap::new(),
                    },
                    metadata: BuildMetadata {
                        invocation_id: "inv_123".to_string(),
                        execution_duration_ms: 100,
                        hermetic_guarantee: true,
                    },
                    byproducts: Vec::new(),
                },
            },
        };

        let envelope =
            AttestationSigner::sign_statement(&statement, &key, "apple-builder-key-1").unwrap();
        assert_eq!(envelope.key_id, "apple-builder-key-1");
        assert!(AttestationSigner::verify_envelope(&envelope, &key).unwrap());

        let wrong_key = [99u8; 32];
        assert!(!AttestationSigner::verify_envelope(&envelope, &wrong_key).unwrap());
    }
}
