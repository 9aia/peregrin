use std::future::Future;
use std::pin::Pin;

use crate::Context;
use crate::Error;
use crate::Operation;
use ac_ast_canonical::Expr;
use ac_runtime_values::NonFunctionValue;

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

    /// Resolve a name to a typed [`Operation`].
    ///
    /// Returns `None` by default. Override to provide operations that
    /// signal (yield) instead of completing. The VM checks this before
    /// [`Host::function`] — if a name resolves to an operation, the
    /// operation's `perform` method is called instead of a function's
    /// `call` method.
    ///
    /// Operations enable algebraic effect handling: the VM captures
    /// the continuation when an operation signals, and a handler can
    /// intercept, inspect the arguments, and resume.
    fn operation(&self, _name: &str) -> Option<&dyn Operation> {
        None
    }

    /// Resolve a function name to a [`LazyFunction`] that receives
    /// unevaluated arguments.
    ///
    /// Returns `None` by default. Override to provide functions that
    /// need lazy (short-circuit) argument evaluation — e.g. `if`,
    /// `and`, `or`. The VM checks this before [`Host::function`] and
    /// [`Host::operation`]. If a name resolves to a lazy function,
    /// the VM passes the raw expression tree instead of pre-evaluated
    /// values, and the function decides which arguments to evaluate.
    fn lazy_function(&self, _name: &str) -> Option<&dyn LazyFunction> {
        None
    }

    /// Resolve a function name to a [`GenericFunction`] that receives
    /// resolved type arguments.
    ///
    /// Returns `None` by default. The VM checks this when a call carries
    /// non-empty `type_args`. If a generic function is registered, it
    /// receives the type arguments via [`GenericFunction::call_with_types`].
    /// Otherwise, the VM falls back to [`Host::function`].
    fn generic_function(&self, _name: &str) -> Option<&dyn GenericFunction> {
        None
    }
}

/// A callable function provided by the host.
///
/// Functions implement the execution semantics for built-in
/// operations. The VM evaluates arguments and passes the resulting
/// [`NonFunctionValue`]s to the function's [`call`](Function::call) method.
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
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error>;
}

/// A generic function that receives resolved type arguments at runtime.
///
/// Extends [`Function`] with the ability to inspect the concrete types
/// that a generic function was instantiated with. The typechecker
/// populates `type_args` during monomorphization — each entry is a
/// string representation of the concrete type (e.g. `"string"`,
/// `"array<number>"`).
///
/// The VM checks [`Host::generic_function`] for calls that carry
/// type arguments. If a generic function is registered, it receives
/// the type arguments. Otherwise, the VM falls back to [`Function`].
///
/// # Example
///
/// ```ignore
/// struct TypedIdentity;
///
/// impl GenericFunction for TypedIdentity {
///     fn call_with_types(
///         &self,
///         args: &[NonFunctionValue],
///         type_args: &[String],
///         _ctx: &mut dyn Context,
///     ) -> Result<NonFunctionValue, Error> {
///         // type_args[0] might be "string" or "number"
///         Ok(args[0].clone())
///     }
/// }
/// ```
pub trait GenericFunction: Function {
    /// Call this function with pre-evaluated arguments and resolved type arguments.
    ///
    /// - `args` — the evaluated argument values, in order.
    /// - `type_args` — resolved concrete type names from the typechecker.
    /// - `ctx` — the execution context.
    fn call_with_types(
        &self,
        args: &[NonFunctionValue],
        type_args: &[String],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error>;
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
///         args: &'a [NonFunctionValue],
///         _ctx: &'a mut (dyn Context + 'a),
///     ) -> Pin<Box<dyn Future<Output = Result<NonFunctionValue, Error>> + 'a>> {
///         Box::pin(async {
///             // async work here
///             Ok(NonFunctionValue::new("result"))
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
        args: &'a [NonFunctionValue],
        ctx: &'a mut (dyn Context + 'a),
    ) -> Pin<Box<dyn Future<Output = Result<NonFunctionValue, Error>> + 'a>>;
}

/// A lazy function that receives unevaluated arguments.
///
/// Unlike [`Function`], which receives pre-evaluated argument values,
/// `LazyFunction` receives the raw expression tree. The function
/// decides which arguments to evaluate and in what order by calling
/// the `evaluate` closure.
///
/// This enables short-circuit evaluation:
/// - `if(cond, then, else)` only evaluates the chosen branch.
/// - `and(a, b)` stops at the first `false`.
/// - `or(a, b)` stops at the first `true`.
///
/// The VM checks [`Host::lazy_function`] before [`Host::function`]
/// and [`Host::operation`]. If a name resolves to a lazy function,
/// the VM calls it with unevaluated args. Otherwise, the VM eagerly
/// evaluates args and dispatches to [`Host::function`].
///
/// # Implementing
///
/// A lazy function can also implement [`Function`] for direct callers
/// that pass pre-evaluated values. The `Function` impl evaluates all
/// args eagerly; the `LazyFunction` impl short-circuits.
///
/// ```ignore
/// struct If;
///
/// impl LazyFunction for If {
///     fn call(
///         &self,
///         args: &[Expr<NonFunctionValue>],
///         ctx: &mut dyn Context,
///         evaluate: &mut dyn FnMut(&Expr<NonFunctionValue>, &mut dyn Context) -> Result<NonFunctionValue, Error>,
///     ) -> Result<NonFunctionValue, Error> {
///         let cond = evaluate(&args[0], ctx)?.as_bool().unwrap_or(false);
///         if cond {
///             evaluate(&args[1], ctx)
///         } else {
///             evaluate(&args[2], ctx)
///         }
///     }
/// }
/// ```
pub trait LazyFunction {
    /// Call this function with unevaluated arguments.
    ///
    /// # Arguments
    ///
    /// - `args` — the unevaluated argument expressions, in order.
    /// - `ctx` — the execution context, for variable access.
    /// - `evaluate` — a closure that evaluates a sub-expression in the
    ///   given context. Call this to evaluate specific arguments on
    ///   demand.
    ///
    /// # Returns
    ///
    /// The result value, or an [`Error`] if execution failed.
    fn call(
        &self,
        args: &[Expr<NonFunctionValue>],
        ctx: &mut dyn Context,
        evaluate: &mut dyn FnMut(
            &Expr<NonFunctionValue>,
            &mut dyn Context,
        ) -> Result<NonFunctionValue, Error>,
    ) -> Result<NonFunctionValue, Error>;
}

/// A borrowed reference to a [`Host`] that itself implements [`Host`].
///
/// This lets the generic [`Vm`] run against a `&dyn Host` without
/// needing ownership of the host value.
#[derive(Clone, Copy)]
pub struct HostRef<'a>(pub &'a dyn Host);

impl<'a> HostRef<'a> {
    /// Wraps a `&dyn Host` for use with the VM.
    pub fn new(host: &'a dyn Host) -> Self {
        HostRef(host)
    }
}

impl<'a> Host for HostRef<'a> {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        self.0.function(name)
    }
    fn async_function(&self, name: &str) -> Option<&dyn AsyncFunction> {
        self.0.async_function(name)
    }
    fn operation(&self, name: &str) -> Option<&dyn Operation> {
        self.0.operation(name)
    }
    fn lazy_function(&self, name: &str) -> Option<&dyn LazyFunction> {
        self.0.lazy_function(name)
    }
    fn generic_function(&self, name: &str) -> Option<&dyn GenericFunction> {
        self.0.generic_function(name)
    }
}
