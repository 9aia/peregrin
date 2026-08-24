use sha2::{Digest, Sha256};

use crate::canonical::canonical_string;

/// A content hash identifying a definition.
///
/// Uses SHA-256, hex-encoded. The hash is computed from the canonical
/// JSON representation of a definition — sorted keys, no whitespace.
///
/// Two definitions with the same canonical form produce the same hash,
/// regardless of formatting or key order in the source.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DefinitionHash(String);

impl DefinitionHash {
    /// Computes a hash from a canonical JSON string.
    pub fn from_canonical_string(canonical: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        let result = hasher.finalize();
        DefinitionHash(hex::encode(result))
    }

    /// Computes a hash from a JSON value (canonicalized first).
    pub fn from_json(value: &serde_json::Value) -> Self {
        let canonical = canonical_string(value);
        Self::from_canonical_string(&canonical)
    }

    /// Creates a `DefinitionHash` from a raw hex string.
    ///
    /// Does not validate the hash — use with care.
    pub fn from_hex(hex: impl Into<String>) -> Self {
        DefinitionHash(hex.into())
    }

    /// Returns the hex-encoded hash string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the hex-encoded hash string, consuming self.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for DefinitionHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for DefinitionHash {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()) {
            Ok(DefinitionHash(s.to_string()))
        } else {
            Err(format!(
                "invalid definition hash: expected 64 hex chars, got {s}"
            ))
        }
    }
}
