use crate::entry::ArchiveEntry;
use crate::error::ArchiveError;

pub trait Archive: Send + Sync {
    fn pack(&self, entries: &[ArchiveEntry]) -> Result<Vec<u8>, ArchiveError>;
    fn unpack(&self, input: &[u8]) -> Result<Vec<ArchiveEntry>, ArchiveError>;
}
