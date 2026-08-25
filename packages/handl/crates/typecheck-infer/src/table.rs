use std::collections::BTreeMap;

use handl_typecheck_core::{FunctionType, SchemaInterpreter, Type, TypeParam};
use serde_json::Value as JsonValue;

/// A raw function signature: `(name, type_params, params, return_schema)`.
///
/// `type_params` is a list of type parameter declarations (e.g. `[TypeParam::new("T")]`).
/// `params` is a list of `(param_name, raw_schema)` pairs.
/// `return_schema` is the raw return type schema, if declared.
pub type RawFnSig = (
    String,
    Vec<TypeParam>,
    Vec<(String, JsonValue)>,
    Option<JsonValue>,
);

/// Host function signatures for typechecking.
///
/// Passed to the typechecker via `Config::with_extension`. The runner
/// merges these with document-declared function signatures to build
/// the full `FunctionTable`.
///
/// # Example
/// ```ignore
/// use handl_compiler::{Config, typecheck_only_config};
/// use handl_typecheck_infer::HostFunctionSigs;
///
/// let config = typecheck_only_config().with_extension(HostFunctionSigs::default_host());
/// ```
#[derive(Debug, Clone)]
pub struct HostFunctionSigs(pub Vec<RawFnSig>);

impl HostFunctionSigs {
    /// Default host function signatures for the reference runtime.
    ///
    /// These mirror `handl_vm_host_canonical::CanonicalHost` (currently `var` and
    /// `resolve`).
    /// Also includes generic signatures for higher-order array functions
    /// (`array.map`, `array.filter`, `array.reduce`) that are used by
    /// `handl-lib-utils`.
    pub fn default_host() -> Self {
        use serde_json::json;
        HostFunctionSigs(vec![
            // ── generic higher-order array functions ──────────────
            // array.map<T, U>(fn: (T) -> U, arr: Array<T>) -> Array<U>
            (
                "array.map".to_string(),
                vec![TypeParam::new("T"), TypeParam::new("U")],
                vec![
                    (
                        "fn".to_string(),
                        json!({
                            "type": "function",
                            "params": [{ "name": "x", "schema": { "type": "var", "name": "T" } }],
                            "return": { "type": "var", "name": "U" }
                        }),
                    ),
                    (
                        "arr".to_string(),
                        json!({
                            "type": "array",
                            "items": { "type": "var", "name": "T" }
                        }),
                    ),
                ],
                Some(json!({
                    "type": "array",
                    "items": { "type": "var", "name": "U" }
                })),
            ),
            // array.filter<T>(fn: (T) -> Boolean, arr: Array<T>) -> Array<T>
            (
                "array.filter".to_string(),
                vec![TypeParam::new("T")],
                vec![
                    (
                        "fn".to_string(),
                        json!({
                            "type": "function",
                            "params": [{ "name": "x", "schema": { "type": "var", "name": "T" } }],
                            "return": { "type": "boolean" }
                        }),
                    ),
                    (
                        "arr".to_string(),
                        json!({
                            "type": "array",
                            "items": { "type": "var", "name": "T" }
                        }),
                    ),
                ],
                Some(json!({
                    "type": "array",
                    "items": { "type": "var", "name": "T" }
                })),
            ),
            // array.reduce<T, U>(fn: (U, T) -> U, arr: Array<T>, init: U) -> U
            (
                "array.reduce".to_string(),
                vec![TypeParam::new("T"), TypeParam::new("U")],
                vec![
                    (
                        "fn".to_string(),
                        json!({
                            "type": "function",
                            "params": [
                                { "name": "acc", "schema": { "type": "var", "name": "U" } },
                                { "name": "x", "schema": { "type": "var", "name": "T" } }
                            ],
                            "return": { "type": "var", "name": "U" }
                        }),
                    ),
                    (
                        "arr".to_string(),
                        json!({
                            "type": "array",
                            "items": { "type": "var", "name": "T" }
                        }),
                    ),
                    ("init".to_string(), json!({ "type": "var", "name": "U" })),
                ],
                Some(json!({ "type": "var", "name": "U" })),
            ),
            // array.flat_map<T, U>(fn: (T) -> Array<U>, arr: Array<T>) -> Array<U>
            (
                "array.flat_map".to_string(),
                vec![TypeParam::new("T"), TypeParam::new("U")],
                vec![
                    (
                        "fn".to_string(),
                        json!({
                            "type": "function",
                            "params": [{ "name": "x", "schema": { "type": "var", "name": "T" } }],
                            "return": { "type": "array", "items": { "type": "var", "name": "U" } }
                        }),
                    ),
                    (
                        "arr".to_string(),
                        json!({
                            "type": "array",
                            "items": { "type": "var", "name": "T" }
                        }),
                    ),
                ],
                Some(json!({
                    "type": "array",
                    "items": { "type": "var", "name": "U" }
                })),
            ),
            // array.find<T>(fn: (T) -> Boolean, arr: Array<T>) -> T
            (
                "array.find".to_string(),
                vec![TypeParam::new("T")],
                vec![
                    (
                        "fn".to_string(),
                        json!({
                            "type": "function",
                            "params": [{ "name": "x", "schema": { "type": "var", "name": "T" } }],
                            "return": { "type": "boolean" }
                        }),
                    ),
                    (
                        "arr".to_string(),
                        json!({
                            "type": "array",
                            "items": { "type": "var", "name": "T" }
                        }),
                    ),
                ],
                Some(json!({ "type": "var", "name": "T" })),
            ),
            // array.some<T>(fn: (T) -> Boolean, arr: Array<T>) -> Boolean
            (
                "array.some".to_string(),
                vec![TypeParam::new("T")],
                vec![
                    (
                        "fn".to_string(),
                        json!({
                            "type": "function",
                            "params": [{ "name": "x", "schema": { "type": "var", "name": "T" } }],
                            "return": { "type": "boolean" }
                        }),
                    ),
                    (
                        "arr".to_string(),
                        json!({
                            "type": "array",
                            "items": { "type": "var", "name": "T" }
                        }),
                    ),
                ],
                Some(json!({ "type": "boolean" })),
            ),
            // array.every<T>(fn: (T) -> Boolean, arr: Array<T>) -> Boolean
            (
                "array.every".to_string(),
                vec![TypeParam::new("T")],
                vec![
                    (
                        "fn".to_string(),
                        json!({
                            "type": "function",
                            "params": [{ "name": "x", "schema": { "type": "var", "name": "T" } }],
                            "return": { "type": "boolean" }
                        }),
                    ),
                    (
                        "arr".to_string(),
                        json!({
                            "type": "array",
                            "items": { "type": "var", "name": "T" }
                        }),
                    ),
                ],
                Some(json!({ "type": "boolean" })),
            ),
            // array.group_by<T, K>(fn: (T) -> K, arr: Array<T>) -> Object
            (
                "array.group_by".to_string(),
                vec![TypeParam::new("T"), TypeParam::new("K")],
                vec![
                    (
                        "fn".to_string(),
                        json!({
                            "type": "function",
                            "params": [{ "name": "x", "schema": { "type": "var", "name": "T" } }],
                            "return": { "type": "var", "name": "K" }
                        }),
                    ),
                    (
                        "arr".to_string(),
                        json!({
                            "type": "array",
                            "items": { "type": "var", "name": "T" }
                        }),
                    ),
                ],
                Some(json!({ "type": "object" })),
            ),
            // array.partition<T>(fn: (T) -> Boolean, arr: Array<T>) -> Array<Array<T>>
            (
                "array.partition".to_string(),
                vec![TypeParam::new("T")],
                vec![
                    (
                        "fn".to_string(),
                        json!({
                            "type": "function",
                            "params": [{ "name": "x", "schema": { "type": "var", "name": "T" } }],
                            "return": { "type": "boolean" }
                        }),
                    ),
                    (
                        "arr".to_string(),
                        json!({
                            "type": "array",
                            "items": { "type": "var", "name": "T" }
                        }),
                    ),
                ],
                Some(json!({
                    "type": "array",
                    "items": { "type": "array", "items": { "type": "var", "name": "T" } }
                })),
            ),
            // array.sort_by<T>(fn: (T) -> Number, arr: Array<T>) -> Array<T>
            (
                "array.sort_by".to_string(),
                vec![TypeParam::new("T")],
                vec![
                    (
                        "fn".to_string(),
                        json!({
                            "type": "function",
                            "params": [{ "name": "x", "schema": { "type": "var", "name": "T" } }],
                            "return": { "type": "number" }
                        }),
                    ),
                    (
                        "arr".to_string(),
                        json!({
                            "type": "array",
                            "items": { "type": "var", "name": "T" }
                        }),
                    ),
                ],
                Some(json!({
                    "type": "array",
                    "items": { "type": "var", "name": "T" }
                })),
            ),
            // array.sort_by_desc<T>(fn: (T) -> Number, arr: Array<T>) -> Array<T>
            (
                "array.sort_by_desc".to_string(),
                vec![TypeParam::new("T")],
                vec![
                    (
                        "fn".to_string(),
                        json!({
                            "type": "function",
                            "params": [{ "name": "x", "schema": { "type": "var", "name": "T" } }],
                            "return": { "type": "number" }
                        }),
                    ),
                    (
                        "arr".to_string(),
                        json!({
                            "type": "array",
                            "items": { "type": "var", "name": "T" }
                        }),
                    ),
                ],
                Some(json!({
                    "type": "array",
                    "items": { "type": "var", "name": "T" }
                })),
            ),
            // array.compact<T>(arr: Array<T>) -> Array<T>
            (
                "array.compact".to_string(),
                vec![TypeParam::new("T")],
                vec![(
                    "arr".to_string(),
                    json!({
                        "type": "array",
                        "items": { "type": "var", "name": "T" }
                    }),
                )],
                Some(json!({
                    "type": "array",
                    "items": { "type": "var", "name": "T" }
                })),
            ),
            // array.pluck<T>(arr: Array<Object>, key: String) -> Array<T>
            (
                "array.pluck".to_string(),
                vec![TypeParam::new("T")],
                vec![
                    (
                        "arr".to_string(),
                        json!({ "type": "array", "items": { "type": "object" } }),
                    ),
                    ("key".to_string(), json!({ "type": "string" })),
                ],
                Some(json!({
                    "type": "array",
                    "items": { "type": "var", "name": "T" }
                })),
            ),
            // array.min_by<T>(fn: (T) -> Number, arr: Array<T>) -> T
            (
                "array.min_by".to_string(),
                vec![TypeParam::new("T")],
                vec![
                    (
                        "fn".to_string(),
                        json!({
                            "type": "function",
                            "params": [{ "name": "x", "schema": { "type": "var", "name": "T" } }],
                            "return": { "type": "number" }
                        }),
                    ),
                    (
                        "arr".to_string(),
                        json!({
                            "type": "array",
                            "items": { "type": "var", "name": "T" }
                        }),
                    ),
                ],
                Some(json!({ "type": "var", "name": "T" })),
            ),
            // array.max_by<T>(fn: (T) -> Number, arr: Array<T>) -> T
            (
                "array.max_by".to_string(),
                vec![TypeParam::new("T")],
                vec![
                    (
                        "fn".to_string(),
                        json!({
                            "type": "function",
                            "params": [{ "name": "x", "schema": { "type": "var", "name": "T" } }],
                            "return": { "type": "number" }
                        }),
                    ),
                    (
                        "arr".to_string(),
                        json!({
                            "type": "array",
                            "items": { "type": "var", "name": "T" }
                        }),
                    ),
                ],
                Some(json!({ "type": "var", "name": "T" })),
            ),
            // array.count_by<T, K>(fn: (T) -> K, arr: Array<T>) -> Object
            (
                "array.count_by".to_string(),
                vec![TypeParam::new("T"), TypeParam::new("K")],
                vec![
                    (
                        "fn".to_string(),
                        json!({
                            "type": "function",
                            "params": [{ "name": "x", "schema": { "type": "var", "name": "T" } }],
                            "return": { "type": "var", "name": "K" }
                        }),
                    ),
                    (
                        "arr".to_string(),
                        json!({
                            "type": "array",
                            "items": { "type": "var", "name": "T" }
                        }),
                    ),
                ],
                Some(json!({ "type": "object" })),
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
