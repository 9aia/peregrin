#[derive(Debug, Clone, PartialEq)]
pub struct ArchiveEntry {
    pub path: String,
    pub contents: Vec<u8>,
}

impl ArchiveEntry {
    pub fn new(path: impl Into<String>, contents: impl Into<Vec<u8>>) -> Self {
        Self {
            path: path.into(),
            contents: contents.into(),
        }
    }
}
