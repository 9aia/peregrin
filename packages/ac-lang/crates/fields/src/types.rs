use std::collections::HashMap;

use ac_runtime::RuntimeValue;
use serde_json::Value as JsonValue;

/// The conventional name of the default namespace used for shorthand
/// path resolution (bare `name` → `DEFAULT_NAMESPACE.name`).
///
/// This is a runtime host convention, not a language-level requirement.
/// A runtime host may use a different name by inserting it into the
/// namespaces map under a different key and adjusting shorthand resolution
/// accordingly.
pub const DEFAULT_NAMESPACE: &str = "doc";

/// A collection of named namespaces for `${ns.key}` interpolation.
///
/// The default namespace (conventionally `doc`) is typically built from
/// the document's own data fields. `env` is typically built from
/// environment variables. Custom namespaces can be added by the caller.
/// Which namespaces exist is a runtime host decision.
pub type Namespaces = HashMap<String, JsonValue>;

/// Resolves function calls during interpolation.
///
/// Implementations typically wrap a VM + Host. The resolver receives
/// pre-validated arguments (if param schemas are declared) and returns
/// the function's result.
pub trait FunctionResolver {
    /// Calls `name` with `args`, returning the result.
    fn call(&self, name: &str, args: &[JsonValue]) -> Result<JsonValue, String>;
}

/// A `FunctionResolver` that does nothing — all calls fail.
/// Useful when no functions are needed.
pub struct NoFunctions;

impl FunctionResolver for NoFunctions {
    fn call(&self, name: &str, _args: &[JsonValue]) -> Result<JsonValue, String> {
        Err(format!("unknown function: {name}"))
    }
}

/// Declares how a computed field's `arg` is typed.
#[derive(Debug, Clone)]
pub enum ArgSpec {
    /// The arg is available without type validation.
    Untyped,
    /// The arg is validated against this raw schema value before the expr runs.
    Typed(JsonValue),
}

/// A computed field declaration extracted from `fields:`.
///
/// Fields with an `expr:` key are computed after interpolation.
/// The `arg` field, if present, makes the field's own data value
/// available as `${arg}` inside the expression.
#[derive(Debug, Clone)]
pub struct ComputedField {
    /// The field name.
    pub name: String,
    /// The expression string to evaluate.
    pub expr: String,
    /// If present, the field's data value is available as `${arg}`.
    pub arg: Option<ArgSpec>,
}

impl From<(String, String)> for ComputedField {
    fn from((name, expr): (String, String)) -> Self {
        ComputedField { name, expr, arg: None }
    }
}

/// A function signature declared in the `functions:` section.
///
/// Each param has a name and a raw schema value. The return type
/// also has a raw schema value. Callers construct `Schema<V>`
/// implementations from these raw values.
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    /// `(param_name, raw_schema_value)` pairs, in declaration order.
    pub params: Vec<(String, JsonValue)>,
    /// Raw schema value for the return type, if declared.
    pub return_schema: Option<JsonValue>,
    /// Expression body for user-defined functions, if declared.
    ///
    /// When present, the function is evaluated by parsing this
    /// expression and resolving it via the `FunctionResolver`.
    /// Params are available as bare names (e.g. `x` in `mul(x, 2)`).
    /// When absent, the function must be provided by the host.
    pub body: Option<String>,
}

/// Result of [`extract_fields`]: field schemas, function signatures, and data.
pub struct ExtractedFields {
    /// `(name, raw_schema_value)` pairs for each declared field.
    pub schemas: Vec<(String, JsonValue)>,
    /// `(name, FunctionSignature)` pairs for each declared function.
    pub functions: Vec<(String, FunctionSignature)>,
    /// The document data with `fields:` and `functions:` removed.
    pub data: RuntimeValue,
    /// Field names declared with `inheritable: true`.
    /// These propagate into nested objects during scoped interpolation.
    pub inheritable: Vec<String>,
    /// Field names declared with `mutable: false`.
    /// These cannot be overridden in nested scopes.
    pub immutable: Vec<String>,
    /// `ComputedField` entries for fields with an `expr:` key.
    /// These are computed after interpolation using `compute_fields`.
    pub computed: Vec<ComputedField>,
    /// Field names declared with `merge: true`.
    /// After computation, these fields' values (must be objects) are spread
    /// into the parent scope and the field itself is removed.
    pub merge: Vec<String>,
}
