use base64::{engine::general_purpose, Engine as _};
use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};

use super::common::{byte_array, extract_bytes, require_string, s};
use crate::error::CryptoError;

/// `crypto.base64_encode(data: string | array<number>) → string`
///
/// Encodes the input to a Base64 string using standard padding.
///
/// The input may be a string (UTF-8 bytes) or an array of integers
/// in the range 0–255.
///
/// Examples:
/// - `crypto.base64_encode("hello")` → `"aGVsbG8="`
/// - `crypto.base64_encode([0, 1, 2])` → `"AAEC"`
#[derive(Clone, Copy)]
pub struct Base64Encode;

impl Function for Base64Encode {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        let data = extract_bytes(args, 0)?;
        Ok(s(general_purpose::STANDARD.encode(&data)))
    }
}

/// `crypto.base64_decode(encoded: string) → array<number>`
///
/// Decodes a Base64 string and returns the raw bytes as an array of
/// integers in the range 0–255.
///
/// Returns an error if the input is not valid Base64.
///
/// Examples:
/// - `crypto.base64_decode("aGVsbG8=")` → `[104, 101, 108, 108, 111]`
#[derive(Clone, Copy)]
pub struct Base64Decode;

impl Function for Base64Decode {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        let encoded = require_string(args, 0)?;
        let decoded = general_purpose::STANDARD
            .decode(encoded.as_bytes())
            .map_err(|e| CryptoError::DecodeError { message: e.to_string() })?;
        Ok(byte_array(&decoded))
    }
}

/// `crypto.hex_encode(data: string | array<number>) → string`
///
/// Encodes the input to a lowercase hex-encoded string.
///
/// The input may be a string (UTF-8 bytes) or an array of integers
/// in the range 0–255.
///
/// Examples:
/// - `crypto.hex_encode("hello")` → `"68656c6c6f"`
/// - `crypto.hex_encode([0, 255, 16])` → `"00ff10"`
#[derive(Clone, Copy)]
pub struct HexEncode;

impl Function for HexEncode {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        let data = extract_bytes(args, 0)?;
        Ok(s(hex::encode(&data)))
    }
}

/// `crypto.hex_decode(encoded: string) → array<number>`
///
/// Decodes a hex-encoded string and returns the raw bytes as an array
/// of integers in the range 0–255.
///
/// Returns an error if the input contains non-hex characters or has
/// an odd length.
///
/// Examples:
/// - `crypto.hex_decode("68656c6c6f")` → `[104, 101, 108, 108, 111]`
#[derive(Clone, Copy)]
pub struct HexDecode;

impl Function for HexDecode {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        let encoded = require_string(args, 0)?;
        let decoded = hex::decode(encoded)
            .map_err(|e| CryptoError::DecodeError { message: e.to_string() })?;
        Ok(byte_array(&decoded))
    }
}
