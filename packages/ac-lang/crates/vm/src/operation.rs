//! Typed operations for algebraic effect handling.
//!
//! An [`Operation`] is like a [`Function`](crate::Function) but it
//! **signals** instead of completing. When an operation is performed,
//! the VM captures the current continuation and yields it to the
//! caller. A handler can intercept the operation, inspect the
//! arguments, and decide whether to resume the continuation.
//!
//! This is the foundation for Sanada-style arrow handlers adapted to
//! ac-lang's tree-walking VM. Operations are typed, inspectable, and
//! serializable — the continuation machinery in
//! [`continuation`](crate::continuation) captures the frame stack.
//!
//! ## Operation vs Function
//!
//! - A [`Function`] completes: it returns `Ok(value)` or `Err(error)`.
//! - An [`Operation`] signals: it returns `Err(Error::Operation { ... })`
//!   carrying its name and arguments. The VM captures the continuation
//!   and yields `EvalResult::Yielded`.
//!
//! The VM checks `Host::operation` before `Host::function`. If a name
//! resolves to an operation, the operation's `perform` method is called.
//! If `perform` returns `Err(Error::Operation { ... })`, the VM captures
//! the continuation and yields. If `perform` returns `Ok(value)`, the
//! operation completed without signaling (e.g. a no-op or cached result).

use crate::Error;
use crate::Context;
use ac_runtime_values::NonFunctionValue;

/// The signature of an operation — its name and parameter types.
///
/// Used for introspection and handler matching. Parameter types are
/// raw JSON schema values, consistent with `FunctionValue` params.
#[derive(Debug, Clone, PartialEq)]
pub struct OperationSig {
    /// The operation name (e.g. `"fetch"`, `"read_file"`).
    pub name: String,
    /// `(param_name, raw_schema)` pairs, in declaration order.
    pub params: Vec<(String, serde_json::Value)>,
    /// Raw schema value for the return type, if declared.
    pub return_schema: Option<serde_json::Value>}

/// A typed operation that signals instead of completing.
///
/// When the VM dispatches a call and the name resolves to an operation
/// (via [`Host::operation`](crate::Host::operation)), the operation's
/// [`perform`](Operation::perform) method is called. If it returns
/// `Err(Error::Operation { ... })`, the VM captures the continuation
/// and yields. If it returns `Ok(value)`, the operation completed
/// without signaling.
///
/// # Implementing
///
/// ```ignore
/// struct Fetch;
///
/// impl Operation for Fetch {
///     fn signature(&self) -> OperationSig {
///         OperationSig {
///             name: "fetch".to_string(),
///             params: vec![("url".to_string(), serde_json::json!({"type": "string"}))],
///             return_schema: Some(serde_json::json!({"type": "string"})),
///         }
///     }
///
///     fn perform(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
///         // Signal: yield to the caller with operation name + args
///         Err(Error::Operation {
///             name: "fetch".to_string(),
///             args: args.to_vec(),
///         })
///     }
/// }
/// ```
pub trait Operation {
    /// Returns the signature of this operation.
    fn signature(&self) -> OperationSig;

    /// Perform the operation with pre-evaluated arguments.
    ///
    /// Return `Err(Error::Operation { name, args })` to signal.
    /// Return `Ok(value)` to complete without signaling (e.g. cached).
    fn perform(&self, args: &[NonFunctionValue], ctx: &mut dyn Context) -> Result<NonFunctionValue, Error>;
}
