use std::collections::HashMap;

use ac_runtime::FunctionValue;

use crate::hash::DefinitionHash;

/// A content-addressable store of function definitions.
///
/// Stores [`FunctionValue`]s indexed by their [`DefinitionHash`].
/// The store is the backbone of content-addressing: definitions are
/// identified by hash, not by name. Names are resolved to hashes
/// separately via [`NameRegistry`].
///
/// # Determinism
///
/// The same `FunctionValue` always produces the same hash, so the
/// same definition inserted on two different machines will map to
/// the same entry. This enables cross-machine deduplication and
/// reproducible builds.
#[derive(Debug, Clone, Default)]
pub struct DefinitionStore {
    defs: HashMap<DefinitionHash, FunctionValue>}

impl DefinitionStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        DefinitionStore {
            defs: HashMap::new()}
    }

    /// Inserts a function definition and returns its hash.
    ///
    /// If a definition with the same hash already exists, it is
    /// not replaced (content-addressing means identical content =
    /// identical hash = no-op).
    pub fn insert(&mut self, function: FunctionValue) -> DefinitionHash {
        let json = function.to_json();
        let hash = DefinitionHash::from_json(&json);
        self.defs.entry(hash.clone()).or_insert(function);
        hash
    }

    /// Looks up a definition by hash.
    pub fn get(&self, hash: &DefinitionHash) -> Option<&FunctionValue> {
        self.defs.get(hash)
    }

    /// Returns `true` if the store contains a definition with the
    /// given hash.
    pub fn contains(&self, hash: &DefinitionHash) -> bool {
        self.defs.contains_key(hash)
    }

    /// Returns the number of definitions in the store.
    pub fn len(&self) -> usize {
        self.defs.len()
    }

    /// Returns `true` if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    /// Returns an iterator over all (hash, definition) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&DefinitionHash, &FunctionValue)> {
        self.defs.iter()
    }

    /// Serializes the store to a JSON object mapping hash strings to
    /// function definitions.
    pub fn to_json(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        for (hash, func) in &self.defs {
            obj.insert(hash.as_str().to_string(), func.to_json());
        }
        serde_json::Value::Object(obj)
    }

    /// Deserializes a store from a JSON object produced by
    /// [`to_json`](Self::to_json).
    pub fn from_json(value: &serde_json::Value) -> Self {
        let mut defs = HashMap::new();
        if let Some(obj) = value.as_object() {
            for (hash_str, func_json) in obj {
                if let Some(func) = FunctionValue::from_json(func_json) {
                    let hash = DefinitionHash::from_hex(hash_str.clone());
                    defs.insert(hash, func);
                }
            }
        }
        DefinitionStore { defs }
    }
}

/// Maps names to [`DefinitionHash`]es.
///
/// Names are aliases — the same definition can have multiple names.
/// At compile time, names in expressions are resolved to hashes,
/// so the VM sees hash-encoded strings.
#[derive(Debug, Clone, Default)]
pub struct NameRegistry {
    names: HashMap<String, DefinitionHash>}

impl NameRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        NameRegistry {
            names: HashMap::new()}
    }

    /// Registers a name → hash mapping.
    pub fn register(&mut self, name: impl Into<String>, hash: DefinitionHash) {
        self.names.insert(name.into(), hash);
    }

    /// Resolves a name to its hash.
    pub fn resolve(&self, name: &str) -> Option<&DefinitionHash> {
        self.names.get(name)
    }

    /// Returns `true` if the registry contains the given name.
    pub fn contains(&self, name: &str) -> bool {
        self.names.contains_key(name)
    }

    /// Returns the number of registered names.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Returns `true` if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// Returns an iterator over all (name, hash) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &DefinitionHash)> {
        self.names.iter()
    }

    /// Serializes the registry to a JSON object mapping names to
    /// hash strings.
    pub fn to_json(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        for (name, hash) in &self.names {
            obj.insert(
                name.clone(),
                serde_json::Value::String(hash.as_str().to_string()),
            );
        }
        serde_json::Value::Object(obj)
    }

    /// Deserializes a registry from a JSON object produced by
    /// [`to_json`](Self::to_json).
    pub fn from_json(value: &serde_json::Value) -> Self {
        let mut names = HashMap::new();
        if let Some(obj) = value.as_object() {
            for (name, hash_val) in obj {
                if let Some(hash_str) = hash_val.as_str() {
                    names.insert(name.clone(), DefinitionHash::from_hex(hash_str.to_string()));
                }
            }
        }
        NameRegistry { names }
    }
}
