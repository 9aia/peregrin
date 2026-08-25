use std::io::Write;

use handl_lib_compress::{CompressError, Compressor, Decompressor};
use flate2::Compression;
use flate2::write::{GzDecoder, GzEncoder};

/// Gzip compression/decompression codec.
#[derive(Clone, Copy, Debug, Default)]
pub struct Gzip;

impl Gzip {
    pub fn new() -> Self {
        Self
    }
}

impl Compressor for Gzip {
    fn compress(&self, input: &[u8]) -> Result<Vec<u8>, CompressError> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(input)
            .map_err(|e| CompressError::compress(e.to_string()))?;
        encoder
            .finish()
            .map_err(|e| CompressError::compress(e.to_string()))
    }
}

impl Decompressor for Gzip {
    fn decompress(&self, input: &[u8]) -> Result<Vec<u8>, CompressError> {
        let mut decoder = GzDecoder::new(Vec::new());
        decoder
            .write_all(input)
            .map_err(|e| CompressError::decompress(e.to_string()))?;
        decoder
            .finish()
            .map_err(|e| CompressError::decompress(e.to_string()))
    }
}
