use std::future::Future;
use std::pin::Pin;

use crate::{Error, Value};

/// A callback that evaluates an expression string with variable bindings.
///
/// This trait bridges the VM's [`Function`] trait to the higher-level
/// expression evaluator (parser + resolver). It allows host functions
/// like `fp.map` to evaluate function bodies — expression strings that
/// reference bound parameters.
///
/// The implementor is responsible for parsing the expression string,
/// resolving variable references, and dispatching nested function calls.
///
/// ## Implementing
///
/// Typically implemented in the compiler layer, where the dialect parser
/// and function resolver are available:
///
/// ```ignore
/// impl Eval<MyValue> for ExprEval {
///     fn eval_expr(
///         &self,
///         expr: &str,
///         vars: &[(String, MyValue)],
///     ) -> Result<MyValue, Error<MyValue>> {
///         // Parse expr, bind vars, evaluate, return result
///     }
/// }
/// ```
pub trait Eval<V: Value> {
    /// Evaluate an expression string with the given variable bindings.
    ///
    /// # Arguments
    ///
    /// - `expr` — the expression string (e.g. `"mul(x, 2)"`).
    /// - `vars` — `(name, value)` pairs to bind before evaluation.
    ///
    /// # Returns
    ///
    /// The evaluated value, or an [`Error`] if evaluation failed.
    fn eval_expr(&self, expr: &str, vars: &[(String, V)]) -> Result<V, Error<V>>;
}

/// An async callback that evaluates an expression string with variable bindings.
///
/// Like [`Eval`], but returns a `Future`. This allows async host
/// functions (e.g. `fp.map` with an async predicate) to evaluate
/// expression strings that may themselves contain async calls.
///
/// The async VM ([`VirtualMachine::eval_async`](crate::VirtualMachine::eval_async))
/// uses this when set on a [`Context`](crate::Context) via
/// [`Context::set_async_eval`](crate::Context::set_async_eval).
pub trait AsyncEval<V: Value> {
    /// Evaluate an expression string with the given variable bindings.
    ///
    /// Returns a boxed future that resolves to the evaluated
    /// value or an [`Error`].
    fn eval_expr_async<'a>(
        &'a self,
        expr: &'a str,
        vars: &'a [(String, V)],
    ) -> Pin<Box<dyn Future<Output = Result<V, Error<V>>> + 'a>>;
}
