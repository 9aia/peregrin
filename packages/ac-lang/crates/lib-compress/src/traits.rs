use crate::error::CompressError;

pub trait Compressor: Send + Sync {
    fn compress(&self, input: &[u8]) -> Result<Vec<u8>, CompressError>;
}

pub trait Decompressor: Send + Sync {
    fn decompress(&self, input: &[u8]) -> Result<Vec<u8>, CompressError>;
}

pub trait Codec: Compressor + Decompressor {}

impl<T: Compressor + Decompressor> Codec for T {}
