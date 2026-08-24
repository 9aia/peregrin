use ac_runtime_values::NonFunctionValue;
use std::collections::HashMap;

/// A cache key for a host function call.
///
/// Consists of the function name and the serialized argument values.
/// Two calls with the same name and same argument values produce
/// the same key, enabling cache hits across machines.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallKey {
    /// The function name.
    pub function: String,
    /// Arguments serialized as a JSON array string.
    pub args_json: String,
}

impl CallKey {
    /// Creates a cache key from a function name and evaluated arguments.
    pub fn new(function: &str, args: &[NonFunctionValue]) -> Self {
        let args_json = serde_json::to_string(
            &args
                .iter()
                .map(|v| v.clone().into_inner())
                .collect::<Vec<_>>(),
        )
        .unwrap_or_else(|_| "[]".to_string());
        CallKey {
            function: function.to_string(),
            args_json,
        }
    }
}

/// Creates a cache key for a function call.
pub fn call_key(function: &str, args: &[NonFunctionValue]) -> CallKey {
    CallKey::new(function, args)
}

/// A serializable cache of host function call results.
///
/// Maps `(function_name, args)` keys to result values. The cache
/// is serializable as JSON, enabling migration across machines.
///
/// # Serialization format
///
/// ```json
/// {
///   "add|[1,2]": 3,
///   "fetch|[\\"hello\\"]": "fetched:hello"
/// }
/// ```
///
/// Keys are `"function|args_json"` strings. Values are the raw
/// JSON results.
#[derive(Debug, Clone, Default)]
pub struct CallCache {
    entries: HashMap<CallKey, NonFunctionValue>,
}

impl CallCache {
    /// Creates an empty cache.
    pub fn new() -> Self {
        CallCache {
            entries: HashMap::new(),
        }
    }

    /// Creates a cache from a pre-populated map.
    pub fn from_map(entries: HashMap<CallKey, NonFunctionValue>) -> Self {
        CallCache { entries }
    }

    /// Looks up a cached result by function name and arguments.
    pub fn get(&self, function: &str, args: &[NonFunctionValue]) -> Option<&NonFunctionValue> {
        let key = CallKey::new(function, args);
        self.entries.get(&key)
    }

    /// Looks up a cached result by a pre-computed [`CallKey`].
    pub fn get_by_key(&self, key: &CallKey) -> Option<&NonFunctionValue> {
        self.entries.get(key)
    }

    /// Records a call result in the cache.
    pub fn insert(&mut self, function: &str, args: &[NonFunctionValue], result: NonFunctionValue) {
        let key = CallKey::new(function, args);
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
    /// The format is an object mapping `"function|args_json"` keys
    /// to result values.
    pub fn to_json(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        for (key, value) in &self.entries {
            let composite_key = format!("{}|{}", key.function, key.args_json);
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
                if let Some((function, args_json)) = composite_key.split_once('|') {
                    let key = CallKey {
                        function: function.to_string(),
                        args_json: args_json.to_string(),
                    };
                    entries.insert(key, NonFunctionValue::new(result.clone()));
                }
            }
        }
        CallCache { entries }
    }
}
