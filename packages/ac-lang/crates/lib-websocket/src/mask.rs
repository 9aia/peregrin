/// Apply a 32-bit masking key to a payload in-place.
///
/// This is the XOR masking algorithm described in RFC 6455 Section 5.3.
pub fn apply(payload: &mut [u8], key: [u8; 4]) {
    for (i, b) in payload.iter_mut().enumerate() {
        *b ^= key[i % 4];
    }
}
