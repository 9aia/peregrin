use crate::Context;
use crate::Error;
use ac_runtime::RuntimeValue;

/// Provides functions to the VM.
///
/// The host is the bridge between the language runtime and the
/// outside world. The VM asks the host to resolve function names;
/// the host returns a reference to a [`Function`] that knows how
/// to execute that function.
///
/// The VM has no knowledge of what any function does — it only
/// knows how to call functions that the host provides.
///
/// # Implementing a host
///
/// ```ignore
/// struct MyHost {
///     functions: Vec<(String, Box<dyn Function>)>,
/// }
///
/// impl Host for MyHost {
///     fn function(&self, name: &str) -> Option<&dyn Function> {
///         self.functions
///             .iter()
///             .find(|(n, _)| n == name)
///             .map(|(_, f)| f.as_ref())
///     }
/// }
/// ```
pub trait Host {
    /// Resolve a function name to a callable [`Function`].
    ///
    /// Returns `None` if the name is not recognized.
    fn function(&self, name: &str) -> Option<&dyn Function>;
}

/// A callable function provided by the host.
///
/// Functions implement the execution semantics for built-in
/// operations. The VM evaluates arguments and passes the resulting
/// [`RuntimeValue`]s to the function's [`call`](Function::call) method.
///
/// Functions receive a mutable reference to the [`Context`],
/// allowing them to read or modify variables in the current scope.
///
/// # Interior mutability
///
/// The `call` method takes `&self`, not `&mut self`. If a function
/// needs mutable internal state, use interior mutability (e.g.
/// `RefCell`, `AtomicUsize`).
pub trait Function {
    /// Call this function with pre-evaluated arguments.
    ///
    /// # Arguments
    ///
    /// - `args` — the evaluated argument values, in order.
    /// - `ctx` — the execution context, for variable access.
    ///
    /// # Returns
    ///
    /// The result value, or an [`Error`] if execution failed.
    fn call(&self, args: &[RuntimeValue], ctx: &mut Context) -> Result<RuntimeValue, Error>;
}
