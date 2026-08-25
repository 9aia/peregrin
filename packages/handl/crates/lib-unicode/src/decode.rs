//! Source-text decoding for handl language input.
//!
//! Exposes low-level helpers for converting raw byte streams into a sequence
//! of Unicode scalar values, reporting the exact byte offset where an
//! invalid encoding is detected.

/// Error produced when a byte sequence is not valid Unicode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// The byte at `offset` is not a valid UTF-8 continuation byte.
    InvalidContinuation { offset: usize },
    /// The byte at `offset` begins a multi-byte sequence that is truncated.
    TruncatedSequence { offset: usize },
    /// The byte at `offset` is an overlong or otherwise invalid code point.
    InvalidCodePoint { offset: usize },
    /// A 32-bit value is outside the Unicode scalar range or a surrogate.
    InvalidScalar { offset: usize },
}

/// Iterator that decodes a UTF-8 byte slice into `(usize, char)` pairs,
/// where the `usize` is the starting byte offset of the scalar value.
#[derive(Debug, Clone)]
pub struct Utf8Iterator<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Utf8Iterator<'a> {
    /// Create a new iterator over the provided UTF-8 bytes.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    /// Returns the next valid `(offset, char)` pair or the first decode error.
    pub fn next_scalar(&mut self) -> Result<Option<(usize, char)>, DecodeError> {
        if self.offset >= self.bytes.len() {
            return Ok(None);
        }

        let start = self.offset;
        let b0 = self.bytes[start];

        let (len, mut scalar): (usize, u32);

        if b0 < 0x80 {
            scalar = u32::from(b0);
            len = 1;
        } else if b0 < 0xC0 {
            return Err(DecodeError::InvalidContinuation { offset: start });
        } else if b0 < 0xE0 {
            scalar = u32::from(b0 & 0x1F);
            len = 2;
        } else if b0 < 0xF0 {
            scalar = u32::from(b0 & 0x0F);
            len = 3;
        } else if b0 < 0xF8 {
            scalar = u32::from(b0 & 0x07);
            len = 4;
        } else {
            return Err(DecodeError::InvalidCodePoint { offset: start });
        }

        if start + len > self.bytes.len() {
            return Err(DecodeError::TruncatedSequence { offset: start });
        }

        for i in 1..len {
            let b = self.bytes[start + i];
            if b & 0xC0 != 0x80 {
                return Err(DecodeError::InvalidContinuation { offset: start + i });
            }
            scalar = (scalar << 6) | u32::from(b & 0x3F);
        }

        // Reject overlong forms, surrogates, and values above U+10FFFF by
        // validating against the UTF-8 encoding rules.
        if let Some(ch) = char::from_u32(scalar) {
            // Verify the encoded length matches the value to catch overlongs.
            let expected_len = match scalar {
                0..=0x7F => 1,
                0x80..=0x7FF => 2,
                0x800..=0xFFFF => 3,
                0x10000..=0x10FFFF => 4,
                _ => return Err(DecodeError::InvalidCodePoint { offset: start }),
            };
            if expected_len != len {
                return Err(DecodeError::InvalidCodePoint { offset: start });
            }
            self.offset += len;
            Ok(Some((start, ch)))
        } else {
            Err(DecodeError::InvalidCodePoint { offset: start })
        }
    }
}

impl Iterator for Utf8Iterator<'_> {
    type Item = Result<(usize, char), DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_scalar().transpose()
    }
}

/// Decode an entire UTF-8 byte slice into a `Vec<char>`.
///
/// Returns the first error encountered, along with the byte offset where the
/// error occurred.
pub fn decode_utf8(bytes: &[u8]) -> Result<Vec<char>, (usize, DecodeError)> {
    let mut out = Vec::with_capacity(bytes.len());
    for item in Utf8Iterator::new(bytes) {
        match item {
            Ok((_, ch)) => out.push(ch),
            Err(err) => return Err((out.len(), err)),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_ascii() {
        let bytes = b"hello";
        let chars = decode_utf8(bytes).unwrap();
        assert_eq!(chars, ['h', 'e', 'l', 'l', 'o']);
    }

    #[test]
    fn decodes_multibyte() {
        let bytes = "é 日本".as_bytes();
        let chars = decode_utf8(bytes).unwrap();
        assert_eq!(chars, ['é', ' ', '日', '本']);
    }

    #[test]
    fn rejects_invalid_continuation() {
        let bytes = [0xC3, 0x20];
        let err = decode_utf8(&bytes).unwrap_err().1;
        assert_eq!(err, DecodeError::InvalidContinuation { offset: 1 });
    }

    #[test]
    fn rejects_overlong() {
        let bytes = [0xC0, 0x80];
        let err = decode_utf8(&bytes).unwrap_err().1;
        assert_eq!(err, DecodeError::InvalidCodePoint { offset: 0 });
    }
}
