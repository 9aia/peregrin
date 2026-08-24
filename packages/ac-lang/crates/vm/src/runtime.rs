use std::future::Future;
use std::pin::Pin;

use ac_ast_canonical::Expr;
use indexmap::IndexMap;

use crate::{AsyncEval, Error, Eval, Value};

/// The execution context for a VM evaluation.
///
/// A `Context` maintains a stack of variable scopes, resource limits,
/// and optional expression-evaluation callbacks. It is the only thing the
/// VM knows about the execution environment; concrete context types can
/// implement this trait with different backing stores or policies.
pub trait Context<V: Value> {
    /// Defines a variable in the current (topmost) scope.
    fn define(&mut self, name: &str, value: V);

    /// Looks up a variable by name, searching from the innermost
    /// scope outward.
    fn get(&self, name: &str) -> Result<V, Error<V>>;

    /// Pushes a new empty scope onto the scope stack.
    fn push_scope(&mut self);

    /// Pops the topmost scope from the stack.
    fn pop_scope(&mut self) -> Option<IndexMap<String, V>>;

    /// Returns the current scope depth.
    fn depth(&self) -> usize;

    /// Returns the number of calls made so far in this context.
    fn call_count(&self) -> usize;

    /// Increments the call counter. Called by the VM on each `Call`.
    fn increment_call_count(&self) -> Result<(), Error<V>>;

    /// Checks whether the current scope depth is within the limit.
    fn check_depth(&self) -> Result<(), Error<V>>;

    /// Returns `true` if a synchronous eval callback is set.
    fn has_eval(&self) -> bool;

    /// Returns `true` if an asynchronous eval callback is set.
    fn has_async_eval(&self) -> bool;

    /// Sets the synchronous eval callback.
    fn set_eval(&mut self, eval: Box<dyn Eval<V>>);

    /// Sets the asynchronous eval callback.
    fn set_async_eval(&mut self, eval: Box<dyn AsyncEval<V>>);

    /// Evaluates an expression string with the given variable bindings.
    fn eval_expr(&self, expr: &str, vars: &[(String, V)]) -> Result<V, Error<V>>;

    /// Evaluates an expression string asynchronously with the given
    /// variable bindings.
    fn eval_expr_async<'a>(
        &'a self,
        expr: &'a str,
        vars: &'a [(String, V)],
    ) -> Pin<Box<dyn Future<Output = Result<V, Error<V>>> + 'a>>;

    /// Sets the maximum scope depth allowed during evaluation.
    fn set_max_depth(&mut self, max: usize);

    /// Sets the maximum number of function calls allowed during evaluation.
    fn set_max_calls(&mut self, max: usize);

    /// Returns the configured max scope depth, if any.
    fn max_depth(&self) -> Option<usize>;

    /// Returns the configured max call count, if any.
    fn max_calls(&self) -> Option<usize>;

    /// Collects all variables from all scopes.
    fn collect_vars(&self) -> Vec<(String, V)>;
}

/// An abstract virtual-machine evaluator.
///
/// Implementors transform an [`Expr`] tree into a runtime value inside a
/// [`Context`]. The canonical tree-walking implementation is
/// `ac_runtime_canonical::CanonicalVm`.
pub trait VirtualMachine<V: Value> {
    /// Evaluates an expression synchronously.
    fn eval(&self, expr: &Expr<V>, ctx: &mut dyn Context<V>) -> Result<V, Error<V>>;

    /// Evaluates an expression asynchronously.
    fn eval_async(
        &self,
        expr: &Expr<V>,
        ctx: &mut dyn Context<V>,
    ) -> impl Future<Output = Result<V, Error<V>>>;
}
