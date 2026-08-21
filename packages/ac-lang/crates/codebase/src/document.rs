use ac_runtime::FunctionValue;
use ac_store::DefinitionHash;

/// A named entry in a document — a definition with its local name.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentEntry {
    /// The local name within the document (e.g. `"double"`).
    pub name: String,
    /// The content hash of the definition.
    pub hash: DefinitionHash,
    /// The function definition itself.
    pub function: FunctionValue,
}

/// A document — a collection of named definitions.
///
/// Documents are the unit of content-addressing at the codebase level.
/// A document contains named entries, each of which is a function
/// definition with a content hash. The document itself is hashed to
/// produce a [`DocumentHash`], which can be used to import the entire
/// document by hash.
///
/// # Imports
///
/// A document can declare imports — hashes of other documents whose
/// definitions are available for use. This creates a dependency graph
/// that the [`DocumentStore`](crate::DocumentStore) can traverse.
#[derive(Debug, Clone, Default)]
pub struct Document {
    /// The document's title or module name (for human readability).
    pub title: Option<String>,
    /// Named definitions in this document.
    pub entries: Vec<DocumentEntry>,
    /// Hashes of imported documents.
    pub imports: Vec<DefinitionHash>,
}

impl Document {
    /// Creates an empty document.
    pub fn new() -> Self {
        Document::default()
    }

    /// Sets the document title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Adds a named definition to the document.
    pub fn entry(mut self, name: impl Into<String>, function: FunctionValue) -> Self {
        let hash = DefinitionHash::from_json(&function.to_json());
        self.entries.push(DocumentEntry {
            name: name.into(),
            hash,
            function,
        });
        self
    }

    /// Adds an import (hash of another document).
    pub fn import(mut self, hash: DefinitionHash) -> Self {
        self.imports.push(hash);
        self
    }

    /// Looks up an entry by local name.
    pub fn get(&self, name: &str) -> Option<&DocumentEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// Returns the number of entries in the document.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the document has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Serializes the document to canonical JSON for hashing.
    ///
    /// Format:
    /// ```json
    /// {
    ///   "title": "...",
    ///   "entries": [{ "name": "...", "hash": "..." }, ...],
    ///   "imports": ["...", ...]
    /// }
    /// ```
    pub fn to_canonical_json(&self) -> serde_json::Value {
        let mut entries = self
            .entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "name": e.name,
                    "hash": e.hash.as_str(),
                })
            })
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| {
            a.get("name")
                .and_then(|v| v.as_str())
                .cmp(&b.get("name").and_then(|v| v.as_str()))
        });

        let mut imports = self
            .imports
            .iter()
            .map(|h| h.as_str().to_string())
            .collect::<Vec<_>>();
        imports.sort();

        serde_json::json!({
            "title": self.title,
            "entries": entries,
            "imports": imports,
        })
    }

    /// Computes the content hash of this document.
    pub fn hash(&self) -> DefinitionHash {
        let json = self.to_canonical_json();
        DefinitionHash::from_json(&json)
    }
}
