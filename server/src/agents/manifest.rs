use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// TOML-format agent manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub trigger_patterns: Vec<String>,
    pub wasm_hash: String,
}

impl AgentManifest {
    /// Parse from a TOML string.
    pub fn from_toml(s: &str) -> Result<Self, AppError> {
        toml::from_str(s).map_err(|e| AppError::BadRequest(format!("invalid manifest: {e}")))
    }

    /// Validate that all capability strings are known and that wasm_hash is non-empty.
    pub fn validate(&self) -> Result<(), AppError> {
        use crate::agents::capabilities::Capability;
        if self.name.is_empty() {
            return Err(AppError::BadRequest("manifest name is required".into()));
        }
        if self.wasm_hash.is_empty() {
            return Err(AppError::BadRequest("manifest wasm_hash is required".into()));
        }
        for cap in &self.capabilities {
            if Capability::from_str(cap).is_none() {
                return Err(AppError::BadRequest(format!("unknown capability: {cap}")));
            }
        }
        Ok(())
    }

    /// Verify the wasm_hash field matches the SHA-256 of the provided bytes.
    pub fn verify_wasm_hash(&self, wasm_bytes: &[u8]) -> Result<(), AppError> {
        use sha2::{Digest, Sha256};
        let hash = hex::encode(Sha256::digest(wasm_bytes));
        if hash != self.wasm_hash {
            return Err(AppError::BadRequest(format!(
                "wasm_hash mismatch: manifest={}, actual={}",
                self.wasm_hash, hash
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example_toml(wasm_hash: &str) -> String {
        format!(
            r#"
name = "test-agent"
version = "0.1.0"
description = "A test agent"
capabilities = ["read_docs", "write_messages"]
trigger_patterns = ["@test-agent"]
wasm_hash = "{wasm_hash}"
"#
        )
    }

    #[test]
    fn test_parse_valid_manifest() {
        let m = AgentManifest::from_toml(&example_toml("abc123")).unwrap();
        assert_eq!(m.name, "test-agent");
        assert_eq!(m.version, "0.1.0");
        assert_eq!(m.capabilities, vec!["read_docs", "write_messages"]);
        assert_eq!(m.trigger_patterns, vec!["@test-agent"]);
    }

    #[test]
    fn test_parse_invalid_toml() {
        let result = AgentManifest::from_toml("not valid toml ::::");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_capability() {
        let toml = r#"
name = "bad-agent"
version = "0.1.0"
description = "bad"
capabilities = ["not_a_real_capability"]
wasm_hash = "abc"
"#;
        let m = AgentManifest::from_toml(toml).unwrap();
        assert!(m.validate().is_err());
    }

    #[test]
    fn test_validate_empty_name() {
        let toml = r#"
name = ""
version = "0.1.0"
description = "x"
capabilities = []
wasm_hash = "abc"
"#;
        let m = AgentManifest::from_toml(toml).unwrap();
        assert!(m.validate().is_err());
    }

    #[test]
    fn test_verify_wasm_hash() {
        use sha2::{Digest, Sha256};
        let bytes = b"fake wasm content";
        let hash = hex::encode(Sha256::digest(bytes));
        let toml = format!(
            r#"name = "h" version = "1" description = "x" capabilities = [] wasm_hash = "{hash}""#
        );
        // Use inline table syntax since the above is not valid multi-line
        // Just directly create the struct
        let m = AgentManifest {
            name: "h".into(),
            version: "1".into(),
            description: "x".into(),
            capabilities: vec![],
            trigger_patterns: vec![],
            wasm_hash: hash,
        };
        assert!(m.verify_wasm_hash(bytes).is_ok());
    }

    #[test]
    fn test_verify_wasm_hash_mismatch() {
        let m = AgentManifest {
            name: "h".into(),
            version: "1".into(),
            description: "x".into(),
            capabilities: vec![],
            trigger_patterns: vec![],
            wasm_hash: "wrong_hash".into(),
        };
        assert!(m.verify_wasm_hash(b"content").is_err());
    }
}
