use std::io::{Cursor, Read};

use handl_lib_archive::{Archive, ArchiveEntry, ArchiveError};
use tar::{Archive as TarReader, Builder, Header};

/// Tar archive packer/unpacker.
#[derive(Clone, Copy, Debug, Default)]
pub struct TarArchive;

impl TarArchive {
    pub fn new() -> Self {
        Self
    }
}

impl Archive for TarArchive {
    fn pack(&self, entries: &[ArchiveEntry]) -> Result<Vec<u8>, ArchiveError> {
        let mut builder = Builder::new(Vec::new());

        for entry in entries {
            let mut header = Header::new_gnu();
            header.set_size(entry.contents.len().try_into().unwrap_or(u64::MAX));
            let mut data = entry.contents.as_slice();
            builder
                .append_data(&mut header, &entry.path, &mut data)
                .map_err(|e| ArchiveError::pack(e.to_string()))?;
        }

        builder
            .into_inner()
            .map_err(|e| ArchiveError::pack(e.to_string()))
    }

    fn unpack(&self, input: &[u8]) -> Result<Vec<ArchiveEntry>, ArchiveError> {
        let mut archive = TarReader::new(Cursor::new(input));
        let mut entries = Vec::new();

        for entry_result in archive
            .entries()
            .map_err(|e| ArchiveError::unpack(e.to_string()))?
        {
            let mut entry = entry_result.map_err(|e| ArchiveError::unpack(e.to_string()))?;
            let path = entry
                .path()
                .map_err(|e| ArchiveError::invalid_path(e.to_string()))?
                .to_string_lossy()
                .into_owned();
            let mut contents = Vec::new();
            entry
                .read_to_end(&mut contents)
                .map_err(|e| ArchiveError::unpack(e.to_string()))?;
            entries.push(ArchiveEntry { path, contents });
        }

        Ok(entries)
    }
}
