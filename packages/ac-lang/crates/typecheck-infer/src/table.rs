use std::collections::BTreeMap;

use ac_typecheck_core::{FunctionType, SchemaInterpreter, Type};
use serde_json::Value as JsonValue;

/// A raw function signature: `(name, type_params, params, return_schema)`.
///
/// `type_params` is a list of type parameter names (e.g. `["T", "U"]`).
/// `params` is a list of `(param_name, raw_schema)` pairs.
/// `return_schema` is the raw return type schema, if declared.
pub type RawFnSig = (String, Vec<String>, Vec<(String, JsonValue)>, Option<JsonValue>);

/// Host function signatures for typechecking.
///
/// Passed to the typechecker via `Config::with_extension`. The runner
/// merges these with document-declared function signatures to build
/// the full `FunctionTable`.
///
/// # Example
/// ```ignore
/// use ac_compiler::{Config, typecheck_only_config};
/// use ac_typecheck_infer::HostFunctionSigs;
///
/// let config = typecheck_only_config().with_extension(HostFunctionSigs::default_host());
/// ```
#[derive(Debug, Clone)]
pub struct HostFunctionSigs(pub Vec<RawFnSig>);

impl HostFunctionSigs {
    /// Default host function signatures for the reference runtime.
    ///
    /// These mirror `ac_host::DefaultHost` — `add`, `mul`, `concat`, `len`.
    /// All are monomorphic.
    pub fn default_host() -> Self {
        use serde_json::json;
        HostFunctionSigs(vec![
            (
                "add".to_string(),
                vec![],
                vec![
                    ("a".to_string(), json!({ "type": "number" })),
                    ("b".to_string(), json!({ "type": "number" })),
                ],
                Some(json!({ "type": "number" })),
            ),
            (
                "mul".to_string(),
                vec![],
                vec![
                    ("a".to_string(), json!({ "type": "number" })),
                    ("b".to_string(), json!({ "type": "number" })),
                ],
                Some(json!({ "type": "number" })),
            ),
            (
                "concat".to_string(),
                vec![],
                vec![
                    ("a".to_string(), json!({ "type": "string" })),
                    ("b".to_string(), json!({ "type": "string" })),
                ],
                Some(json!({ "type": "string" })),
            ),
            (
                "len".to_string(),
                vec![],
                vec![(
                    "value".to_string(),
                    json!({ "type": ["string", "array", "object"] }),
                )],
                Some(json!({ "type": "number" })),
            ),
        ])
    }
}

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
    /// Each entry is `(function_name, type_params, params, return_schema)` where
    /// `type_params` is a list of type parameter names, `params` is a list of
    /// `(param_name, raw_schema)` pairs, and `return_schema` is the raw return
    /// type schema (if declared).
    pub fn from_signatures(sigs: &[RawFnSig], interpreter: &dyn SchemaInterpreter) -> Self {
        let mut table = FunctionTable::new();
        for (name, type_params, params, return_schema) in sigs {
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
                    type_params: type_params.clone(),
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
