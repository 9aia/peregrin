//! Typed operations for algebraic effect handling.
//!
//! An [`Operation`] is like a [`Function`](crate::Function) but it
//! **signals** instead of completing. When an operation is performed,
//! the VM captures the current continuation and yields it to the
//! caller. A handler can intercept the operation, inspect the
//! arguments, and decide whether to resume the continuation.
//!
//! This is the foundation for Sanada-style arrow handlers adapted to
//! handl's tree-walking VM. Operations are typed, inspectable, and
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

use crate::{Context, Error, Value};

/// The signature of an operation — its name and parameter types.
///
/// Used for introspection and handler matching. Parameter types are
/// raw schema values in the concrete value representation.
#[derive(Debug, Clone, PartialEq)]
pub struct OperationSig<V: Value> {
    /// The operation name (e.g. `"fetch"`, `"read_file"`).
    pub name: String,
    /// `(param_name, raw_schema)` pairs, in declaration order.
    pub params: Vec<(String, V)>,
    /// Raw schema value for the return type, if declared.
    pub return_schema: Option<V>,
}

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
/// impl Operation<MyValue> for Fetch {
///     fn signature(&self) -> OperationSig<MyValue> {
///         OperationSig {
///             name: "fetch".to_string(),
///             params: vec![],
///             return_schema: None,
///         }
///     }
///
///     fn perform(&self, args: &[MyValue], _ctx: &mut dyn Context<MyValue>) -> Result<MyValue, Error<MyValue>> {
///         // Signal: yield to the caller with operation name + args
///         Err(Error::Operation {
///             name: "fetch".to_string(),
///             args: args.to_vec(),
///         })
///     }
/// }
/// ```
pub trait Operation<V: Value> {
    /// Returns the signature of this operation.
    fn signature(&self) -> OperationSig<V>;

    /// Perform the operation with pre-evaluated arguments.
    ///
    /// Return `Err(Error::Operation { name, args })` to signal.
    /// Return `Ok(value)` to complete without signaling (e.g. cached).
    fn perform(&self, args: &[V], ctx: &mut dyn Context<V>) -> Result<V, Error<V>>;
}
