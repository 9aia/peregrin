use ac_runtime_values::NonFunctionValue;
use std::collections::HashMap;

use crate::hash::DefinitionHash;

/// A cache key for an evaluation result.
///
/// Consists of a definition hash and the serialized argument values.
/// Two calls to the same definition with the same arguments produce
/// the same key, enabling memoization across machines.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EvalKey {
    /// The content hash of the definition.
    pub hash: DefinitionHash,
    /// Arguments serialized as a JSON array string.
    pub args_json: String,
}

impl EvalKey {
    /// Creates a cache key from a definition hash and evaluated arguments.
    pub fn new(hash: &DefinitionHash, args: &[NonFunctionValue]) -> Self {
        let args_json = serde_json::to_string(
            &args
                .iter()
                .map(|v| v.clone().into_inner())
                .collect::<Vec<_>>(),
        )
        .unwrap_or_else(|_| "[]".to_string());
        EvalKey {
            hash: hash.clone(),
            args_json,
        }
    }
}

/// A memoization cache for evaluation results.
///
/// Maps `(DefinitionHash, args)` keys to result values. With
/// content-addressed definitions and pure functions, the cache is
/// sound: the same definition hash + same arguments always produces
/// the same result.
///
/// Operations (which signal and may have side effects) are never
/// cached. Only pure function calls and handler bodies are cached.
///
/// The cache is serializable as JSON, enabling persistence across
/// runs and sharing across machines.
#[derive(Debug, Clone, Default)]
pub struct EvalCache {
    entries: HashMap<EvalKey, NonFunctionValue>,
}

impl EvalCache {
    /// Creates an empty cache.
    pub fn new() -> Self {
        EvalCache {
            entries: HashMap::new(),
        }
    }

    /// Looks up a cached result by definition hash and arguments.
    pub fn get(
        &self,
        hash: &DefinitionHash,
        args: &[NonFunctionValue],
    ) -> Option<&NonFunctionValue> {
        let key = EvalKey::new(hash, args);
        self.entries.get(&key)
    }

    /// Looks up a cached result by a pre-computed [`EvalKey`].
    pub fn get_by_key(&self, key: &EvalKey) -> Option<&NonFunctionValue> {
        self.entries.get(key)
    }

    /// Records an evaluation result in the cache.
    pub fn insert(
        &mut self,
        hash: &DefinitionHash,
        args: &[NonFunctionValue],
        result: NonFunctionValue,
    ) {
        let key = EvalKey::new(hash, args);
        self.entries.insert(key, result);
    }

    /// Records an evaluation result with a pre-computed key.
    pub fn insert_with_key(&mut self, key: EvalKey, result: NonFunctionValue) {
        self.entries.insert(key, result);
    }

    /// Returns the number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Serializes the cache to a JSON value.
    ///
    /// Format: `{ "hash|args_json": result, ... }`
    pub fn to_json(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        for (key, value) in &self.entries {
            let composite_key = format!("{}|{}", key.hash, key.args_json);
            obj.insert(composite_key, value.clone().into_inner());
        }
        serde_json::Value::Object(obj)
    }

    /// Deserializes a cache from a JSON value produced by
    /// [`to_json`](Self::to_json).
    pub fn from_json(value: &serde_json::Value) -> Self {
        let mut entries = HashMap::new();
        if let Some(obj) = value.as_object() {
            for (composite_key, result) in obj {
                if let Some((hash_str, args_json)) = composite_key.split_once('|') {
                    let key = EvalKey {
                        hash: DefinitionHash::from_hex(hash_str.to_string()),
                        args_json: args_json.to_string(),
                    };
                    entries.insert(key, NonFunctionValue::new(result.clone()));
                }
            }
        }
        EvalCache { entries }
    }
}
