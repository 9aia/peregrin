//! # ac-runtime-values
//!
//! Re-exports the concrete configuration-language value types from
//! `ac-runtime-config` and defines the format `Frontend` trait.
//!
//! This is the concrete JSON-backed runtime façade. The underlying
//! `ac-runtime` traits are generic over the value type; use
//! `Eval<NonFunctionValue>` etc. to specify the concrete value type.

mod frontend;

pub use ac_runtime_config::{CanonicalContext, FunctionValue, HostValue, NonFunctionValue};
pub use frontend::Frontend;

/// Concrete error type for the JSON-backed runtime.
pub type Error = ac_runtime::Error<NonFunctionValue>;

// Re-export the generic VM traits and value bound so consumers can write
// `Eval<NonFunctionValue>`, `Context<NonFunctionValue>`, etc.
pub use ac_runtime::{
    AsyncEval, AsyncFunction, Context, Eval, Function, GenericFunction, Host, HostRef,
    LazyFunction, Operation, OperationSig, Value, VirtualMachine,
};
