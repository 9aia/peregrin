use std::collections::HashMap;

use handl_store::DefinitionHash;

use crate::document::Document;

/// A content-addressable store of documents.
///
/// Stores [`Document`]s indexed by their content hash
/// ([`DefinitionHash`]). Supports import-by-hash: a document can
/// declare imports (hashes of other documents), and the store
/// resolves the full dependency graph.
///
/// # Deduplication
///
/// Documents with the same content produce the same hash, so
/// identical documents are automatically deduplicated.
#[derive(Debug, Clone, Default)]
pub struct DocumentStore {
    docs: HashMap<DefinitionHash, Document>,
}

impl DocumentStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        DocumentStore {
            docs: HashMap::new(),
        }
    }

    /// Inserts a document and returns its hash.
    pub fn insert(&mut self, doc: Document) -> DefinitionHash {
        let hash = doc.hash();
        self.docs.entry(hash.clone()).or_insert(doc);
        hash
    }

    /// Looks up a document by hash.
    pub fn get(&self, hash: &DefinitionHash) -> Option<&Document> {
        self.docs.get(hash)
    }

    /// Returns `true` if the store contains a document with the
    /// given hash.
    pub fn contains(&self, hash: &DefinitionHash) -> bool {
        self.docs.contains_key(hash)
    }

    /// Returns the number of documents in the store.
    pub fn len(&self) -> usize {
        self.docs.len()
    }

    /// Returns `true` if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    /// Returns an iterator over all (hash, document) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&DefinitionHash, &Document)> {
        self.docs.iter()
    }

    /// Resolves all transitive imports of a document.
    ///
    /// Returns a vector of document hashes in dependency order
    /// (imports first, the document itself last). If any import
    /// is missing from the store, it is skipped.
    pub fn resolve_imports(&self, hash: &DefinitionHash) -> Vec<DefinitionHash> {
        let mut visited = std::collections::HashSet::new();
        let mut result = Vec::new();
        self.resolve_imports_recursive(hash, &mut visited, &mut result);
        result
    }

    fn resolve_imports_recursive(
        &self,
        hash: &DefinitionHash,
        visited: &mut std::collections::HashSet<DefinitionHash>,
        result: &mut Vec<DefinitionHash>,
    ) {
        if visited.contains(hash) {
            return;
        }
        visited.insert(hash.clone());

        if let Some(doc) = self.docs.get(hash) {
            for import in &doc.imports {
                self.resolve_imports_recursive(import, visited, result);
            }
            result.push(hash.clone());
        }
    }
}
