use std::collections::BTreeMap;

use ac_typecheck_core::{FunctionType, SchemaInterpreter, Type};
use serde_json::Value as JsonValue;

/// A raw function signature: `(name, params, return_schema)`.
///
/// `params` is a list of `(param_name, raw_schema)` pairs.
/// `return_schema` is the raw return type schema, if declared.
pub type RawFnSig = (String, Vec<(String, JsonValue)>, Option<JsonValue>);

/// A lookup table for function types — maps function names to their
/// [`FunctionType`] (param types + return type).
///
/// Built from raw function signatures using a [`SchemaInterpreter`]
/// to convert raw schema values into [`Type`]s.
pub struct FunctionTable {
    functions: BTreeMap<String, FunctionType>,
}

impl FunctionTable {
    /// Create an empty table.
    pub fn new() -> Self {
        FunctionTable {
            functions: BTreeMap::new(),
        }
    }

    /// Build a table from raw function signatures.
    ///
    /// Each entry is `(function_name, params, return_schema)` where
    /// `params` is a list of `(param_name, raw_schema)` pairs and
    /// `return_schema` is the raw return type schema (if declared).
    pub fn from_signatures(
        sigs: &[RawFnSig],
        interpreter: &dyn SchemaInterpreter,
    ) -> Self {
        let mut table = FunctionTable::new();
        for (name, params, return_schema) in sigs {
            let param_types: Vec<(String, Type)> = params
                .iter()
                .map(|(pname, schema)| (pname.clone(), interpreter.interpret(schema)))
                .collect();

            let return_type = match return_schema {
                Some(schema) => Box::new(interpreter.interpret(schema)),
                None => Box::new(Type::Unknown),
            };

            table.functions.insert(
                name.clone(),
                FunctionType {
                    params: param_types,
                    return_type,
                },
            );
        }
        table
    }

    /// Insert a function type directly.
    pub fn insert(&mut self, name: impl Into<String>, fn_type: FunctionType) {
        self.functions.insert(name.into(), fn_type);
    }

    /// Look up a function's type by name.
    pub fn lookup(&self, name: &str) -> Option<&FunctionType> {
        self.functions.get(name)
    }
}

impl Default for FunctionTable {
    fn default() -> Self {
        Self::new()
    }
}
