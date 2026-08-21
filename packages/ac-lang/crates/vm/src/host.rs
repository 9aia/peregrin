use std::future::Future;
use std::pin::Pin;

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

    /// Resolve a function name to an async callable [`AsyncFunction`].
    ///
    /// Returns `None` by default. Override to provide async functions
    /// (network I/O, timers, cooperative yielding). The async VM
    /// checks this after [`Host::function`] returns `None`.
    fn async_function(&self, _name: &str) -> Option<&dyn AsyncFunction> {
        None
    }
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

/// An async callable function provided by the host.
///
/// Like [`Function`], but returns a `Future` instead of a direct
/// result. This allows host functions to perform async operations
/// (network I/O, timers, cooperative yielding) without blocking the
/// calling thread.
///
/// The async VM ([`Vm::eval_async`](crate::Vm::eval_async)) checks
/// [`Host::async_function`] after [`Host::function`] returns `None`.
/// This means a host can provide both sync and async functions, and
/// the VM will use whichever is available for a given name.
///
/// # Implementing
///
/// ```ignore
/// struct Fetch;
/// impl AsyncFunction for Fetch {
///     fn call<'a>(
///         &'a self,
///         args: &'a [RuntimeValue],
///         _ctx: &'a mut Context,
///     ) -> Pin<Box<dyn Future<Output = Result<RuntimeValue, Error>> + 'a>> {
///         Box::pin(async {
///             // async work here
///             Ok(RuntimeValue::new("result"))
///         })
///     }
/// }
/// ```
///
/// # Yielding
///
/// An async function can yield control to the executor by awaiting
/// a pending future (e.g. `tokio::task::yield_now().await`). This
/// suspends the entire evaluation without producing a result — the
/// executor reschedules it later.
///
/// To yield a value to the caller (explicit yield), return
/// `Err(Error::Yield(value))`. The caller can inspect the yielded
/// value and decide whether to resume.
pub trait AsyncFunction {
    /// Call this function with pre-evaluated arguments.
    ///
    /// Returns a boxed future that resolves to the result value or
    /// an [`Error`]. The lifetime is tied to `self`, `args`, and `ctx`.
    fn call<'a>(
        &'a self,
        args: &'a [RuntimeValue],
        ctx: &'a mut Context,
    ) -> Pin<Box<dyn Future<Output = Result<RuntimeValue, Error>> + 'a>>;
}
