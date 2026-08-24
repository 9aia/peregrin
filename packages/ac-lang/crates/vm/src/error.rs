use std::fmt;

use crate::Value;

/// Errors that can occur during evaluation.
///
/// All errors are cloneable and comparable, making them suitable
/// for testing and replay scenarios.
#[derive(Debug, Clone, PartialEq)]
pub enum Error<V: Value> {
    /// The function name was not recognized by the host.
    UnknownFunction(String),

    /// The variable name was not found in any scope.
    UnknownVariable(String),

    /// A primitive returned an error during execution.
    PrimitiveError(String),

    /// A value did not match the expected type.
    ///
    /// Carries the expected type name and the actual type name for
    /// diagnostics.
    TypeMismatch {
        expected: &'static str,
        got: &'static str,
    },

    /// The argument count did not match what the primitive expected.
    ArgumentCount { expected: usize, got: usize },

    /// The VM yielded control to the caller.
    ///
    /// A host function signalled a yield. The caller can inspect the
    /// yielded value and decide whether to resume evaluation
    /// by calling `eval_async` again.
    ///
    /// In the async VM, implicit yields also occur at `.await` points
    /// when a host future is not yet ready — these are handled by the
    /// executor and do not produce this variant.
    Yield(V),

    /// A typed operation was performed and signaled.
    ///
    /// An [`Operation`](crate::Operation) returned this error to signal
    /// that it needs a handler to intercept. The VM captures the
    /// continuation and yields `EvalResult::Yielded` with the operation
    /// name and arguments embedded.
    ///
    /// Unlike [`Error::Yield`], this carries structured data (operation
    /// name + evaluated args) that a handler can match on.
    Operation {
        /// The operation name (e.g. `"fetch"`).
        name: String,
        /// The evaluated arguments passed to the operation.
        args: Vec<V>,
    },

    /// A resource limit was exceeded during evaluation.
    ///
    /// Enforced by the VM when the [`Context`](crate::Context) has limits
    /// configured (max scope depth, max call count). Hosts that run
    /// untrusted code — such as a sandbox host — set these limits to
    /// prevent runaway recursion or excessive computation.
    ResourceLimit(String),

    /// A name is reserved for a host module and may not be assigned to.
    ReservedName(String),
}

impl<V: Value> fmt::Display for Error<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnknownFunction(name) => {
                write!(f, "unknown function: {name}")
            }
            Error::UnknownVariable(name) => {
                write!(f, "unknown variable: {name}")
            }
            Error::PrimitiveError(msg) => {
                write!(f, "primitive error: {msg}")
            }
            Error::TypeMismatch { expected, got } => {
                write!(f, "type mismatch: expected {expected}, got {got}")
            }
            Error::ArgumentCount { expected, got } => {
                write!(f, "argument count mismatch: expected {expected}, got {got}")
            }
            Error::Yield(value) => {
                write!(f, "yielded: {value:?}")
            }
            Error::Operation { name, args } => {
                write!(f, "operation {name} signaled with {} args", args.len())
            }
            Error::ResourceLimit(msg) => {
                write!(f, "resource limit exceeded: {msg}")
            }
            Error::ReservedName(name) => {
                write!(f, "reserved name: {name}")
            }
        }
    }
}

impl<V: Value> std::error::Error for Error<V> {}
