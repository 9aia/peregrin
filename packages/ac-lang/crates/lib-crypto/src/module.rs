use ac_runtime::Function;
use ac_runtime_modules::Module;

use super::encoding::{Base64Decode, Base64Encode, HexDecode, HexEncode};
use super::hash::{Md5, Sha256};

/// The `crypto` module — hashing and encoding functions.
///
/// Provides functions callable as `crypto.sha256(data)`,
/// `crypto.md5(data)`, `crypto.base64_encode(data)`,
/// `crypto.base64_decode(encoded)`, `crypto.hex_encode(data)`,
/// `crypto.hex_decode(encoded)` in AC expressions.
///
/// Hashing functions accept a string (UTF-8 bytes) or an array of
/// integers (0–255) and return a lowercase hex-encoded digest.
///
/// Encoding functions accept the same input types for encode operations
/// and return a string. Decode operations accept an encoded string and
/// return an array of integers (0–255).
#[derive(Clone, Copy)]
pub struct CryptoModule;

impl Module for CryptoModule {
    fn name(&self) -> &str {
        "crypto"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "sha256" => Some(&Sha256),
            "md5" => Some(&Md5),
            "base64_encode" => Some(&Base64Encode),
            "base64_decode" => Some(&Base64Decode),
            "hex_encode" => Some(&HexEncode),
            "hex_decode" => Some(&HexDecode),
            _ => None,
        }
    }
}
