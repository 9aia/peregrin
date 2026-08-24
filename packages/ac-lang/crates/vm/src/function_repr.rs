use std::fmt::Debug;

use crate::Value;

/// A pluggable representation of a first-class function value.
///
/// This trait is separate from [`Function`](crate::Function) (the host
/// function trait). It describes how to read the metadata of a
/// function value that is encoded as an ordinary runtime value —
/// parameters, optional return schema, and body expression.
///
/// Implementations live next to the concrete value type. For the
/// default JSON-backed `NonFunctionValue`/`FunctionValue` types, see
/// `ac-runtime-config`.
pub trait FunctionRepr<V: Value>: Debug + Send + Sync + 'static {
    /// The declared parameters as `(name, schema)` pairs.
    fn params(&self) -> Vec<(String, V)>;

    /// The declared return-type schema, if any.
    fn return_schema(&self) -> Option<V>;

    /// The raw body expression string (e.g. `"add(x, 1)"`).
    fn body(&self) -> &str;
}
