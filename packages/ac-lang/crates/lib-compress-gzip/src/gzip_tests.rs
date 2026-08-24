use ac_lib_compress::{Compressor, Decompressor};

use super::Gzip;

#[test]
fn roundtrip() {
    let input = b"hello, compressible world!";
    let compressed = Gzip.compress(input).unwrap();
    assert!(!compressed.is_empty());
    assert!(compressed.len() < 100);

    let decompressed = Gzip.decompress(&compressed).unwrap();
    assert_eq!(decompressed, input);
}

#[test]
fn roundtrip_empty() {
    let input = b"";
    let compressed = Gzip.compress(input).unwrap();
    let decompressed = Gzip.decompress(&compressed).unwrap();
    assert_eq!(decompressed, input);
}

#[test]
fn roundtrip_binary() {
    let input: Vec<u8> = (0..=255).collect();
    let compressed = Gzip.compress(&input).unwrap();
    let decompressed = Gzip.decompress(&compressed).unwrap();
    assert_eq!(decompressed, input);
}

#[test]
fn invalid_data_fails_decompression() {
    let input = b"not gzip";
    assert!(Gzip.decompress(input).is_err());
}
