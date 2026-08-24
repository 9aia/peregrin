use std::cell::Cell;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use ac_runtime::{AsyncEval, Context, Error, Eval};
use indexmap::IndexMap;
use serde_json::Value as JsonValue;

use crate::NonFunctionValue;

/// The execution context for a VM evaluation.
///
/// A context maintains a stack of variable scopes. Each scope is
/// a map of name-value pairs with O(1) lookup. Variable lookup
/// searches from the innermost scope outward, returning the first
/// match.
///
/// Scopes are pushed and popped to model function call boundaries.
/// The bottommost scope is the global scope and is never popped.
///
/// The context may also hold an [`Eval`] callback, which allows host
/// functions to evaluate expression strings (e.g. function bodies).
/// The callback is optional — when not set, [`Context::eval_expr`]
/// returns an error.
pub struct CanonicalContext {
    scopes: Vec<IndexMap<String, NonFunctionValue>>,
    eval: Option<Box<dyn Eval<NonFunctionValue>>>,
    async_eval: Option<Box<dyn AsyncEval<NonFunctionValue>>>,
    max_depth: Option<usize>,
    max_calls: Option<usize>,
    call_count: Cell<usize>,
}

impl CanonicalContext {
    /// Creates a new context with a single empty global scope.
    pub fn new() -> Self {
        CanonicalContext {
            scopes: vec![IndexMap::new()],
            eval: None,
            async_eval: None,
            max_depth: None,
            max_calls: None,
            call_count: Cell::new(0),
        }
    }

    /// Creates a context with each namespace root bound as a variable.
    ///
    /// Also promotes the top-level keys of every object-valued namespace
    /// (conventionally `doc`) to variables, so shorthand references like
    /// `transform` resolve directly.
    pub fn from_namespaces(namespaces: &HashMap<String, JsonValue>) -> Self {
        let mut ctx = Self::new();
        for (name, value) in namespaces {
            ctx.define(name.clone(), NonFunctionValue::new(value.clone()));
        }
        for (name, value) in namespaces {
            if let Some(obj) = value.as_object() {
                for (key, val) in obj {
                    // The key shadows the namespace name if they collide;
                    // this mirrors "name" being a shorthand for "doc.name".
                    if key != name {
                        ctx.define(key.clone(), NonFunctionValue::new(val.clone()));
                    }
                }
            }
        }
        ctx
    }

    /// Sets the [`Eval`] callback for this context.
    ///
    /// When set, host functions can call [`Context::eval_expr`] to
    /// evaluate expression strings with variable bindings. This enables
    /// higher-order functions like `fp.map`, `fp.filter`, `fp.reduce`.
    pub fn set_eval(&mut self, eval: Box<dyn Eval<NonFunctionValue>>) {
        self.eval = Some(eval);
    }

    /// Returns `true` if an [`Eval`] callback is set.
    pub fn has_eval(&self) -> bool {
        self.eval.is_some()
    }

    /// Evaluates an expression string with the given variable bindings.
    ///
    /// Delegates to the [`Eval`] callback set via [`Context::set_eval`].
    /// Returns [`Error::PrimitiveError`] if no callback is set.
    pub fn eval_expr(
        &self,
        expr: &str,
        vars: &[(String, NonFunctionValue)],
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        self.eval
            .as_ref()
            .ok_or(Error::PrimitiveError("no eval callback set".to_string()))?
            .eval_expr(expr, vars)
    }

    /// Sets the [`AsyncEval`] callback for this context.
    ///
    /// When set, async host functions can call [`Context::eval_expr_async`]
    /// to evaluate expression strings with variable bindings asynchronously.
    pub fn set_async_eval(&mut self, eval: Box<dyn AsyncEval<NonFunctionValue>>) {
        self.async_eval = Some(eval);
    }

    /// Returns `true` if an [`AsyncEval`] callback is set.
    pub fn has_async_eval(&self) -> bool {
        self.async_eval.is_some()
    }

    /// Evaluates an expression string asynchronously with the given
    /// variable bindings.
    ///
    /// Delegates to the [`AsyncEval`] callback set via
    /// [`Context::set_async_eval`]. Returns a future that resolves to
    /// the evaluated [`NonFunctionValue`] or an [`Error`].
    pub fn eval_expr_async<'a>(
        &'a self,
        expr: &'a str,
        vars: &'a [(String, NonFunctionValue)],
    ) -> Pin<Box<dyn Future<Output = Result<NonFunctionValue, Error<NonFunctionValue>>> + 'a>> {
        match self.async_eval.as_ref() {
            Some(eval) => eval.eval_expr_async(expr, vars),
            None => Box::pin(std::future::ready(Err(Error::PrimitiveError(
                "no async eval callback set".to_string(),
            )))),
        }
    }

    /// Pushes a new empty scope onto the scope stack.
    pub fn push_scope(&mut self) {
        self.scopes.push(IndexMap::new());
    }

    /// Pops the topmost scope from the stack.
    ///
    /// Returns the popped scope's entries, or `None` if only the
    /// global scope remains (the global scope cannot be popped).
    pub fn pop_scope(&mut self) -> Option<IndexMap<String, NonFunctionValue>> {
        if self.scopes.len() > 1 {
            self.scopes.pop()
        } else {
            None
        }
    }

    /// Returns the current scope depth.
    ///
    /// The global scope is depth 1.
    pub fn depth(&self) -> usize {
        self.scopes.len()
    }

    /// Sets the maximum scope depth allowed during evaluation.
    ///
    /// When set, the VM returns [`Error::ResourceLimit`] if a call
    /// would push the scope depth beyond this limit. This prevents
    /// runaway recursion in untrusted code.
    pub fn set_max_depth(&mut self, max: usize) {
        self.max_depth = Some(max);
    }

    /// Sets the maximum number of function calls allowed during evaluation.
    ///
    /// When set, the VM returns [`Error::ResourceLimit`] after the
    /// call count exceeds this limit.
    pub fn set_max_calls(&mut self, max: usize) {
        self.max_calls = Some(max);
    }

    /// Returns the configured max scope depth, if any.
    pub fn max_depth(&self) -> Option<usize> {
        self.max_depth
    }

    /// Returns the configured max call count, if any.
    pub fn max_calls(&self) -> Option<usize> {
        self.max_calls
    }

    /// Returns the number of calls made so far in this context.
    pub fn call_count(&self) -> usize {
        self.call_count.get()
    }

    /// Increments the call counter. Called by the VM on each `Call`.
    ///
    /// Returns `Err(Error::ResourceLimit)` if the limit is exceeded.
    pub fn increment_call_count(&self) -> Result<(), Error<NonFunctionValue>> {
        let count = self.call_count.get() + 1;
        self.call_count.set(count);
        if let Some(max) = self.max_calls {
            if count > max {
                return Err(Error::ResourceLimit(format!("call limit exceeded: {max}")));
            }
        }
        Ok(())
    }

    /// Checks whether the current scope depth is within the limit.
    ///
    /// Called by the VM before pushing a new scope for a function call.
    /// Returns `Err(Error::ResourceLimit)` if the limit would be exceeded.
    pub fn check_depth(&self) -> Result<(), Error<NonFunctionValue>> {
        if let Some(max) = self.max_depth {
            if self.scopes.len() >= max {
                return Err(Error::ResourceLimit(format!("depth limit exceeded: {max}")));
            }
        }
        Ok(())
    }

    /// Defines a variable in the current (topmost) scope.
    ///
    /// If a variable with the same name exists in the current
    /// scope, it is overwritten.
    pub fn define(&mut self, name: impl Into<String>, value: NonFunctionValue) {
        self.scopes.last_mut().unwrap().insert(name.into(), value);
    }

    /// Looks up a variable by name, searching from the innermost
    /// scope outward.
    ///
    /// Returns a clone of the value if found, or
    /// [`Error::UnknownVariable`] if not.
    pub fn get(&self, name: &str) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Ok(value.clone());
            }
        }
        Err(Error::UnknownVariable(name.to_string()))
    }

    /// Collects all variables from all scopes, with innermost scope
    /// taking precedence on name conflicts.
    ///
    /// Returns a `Vec` of `(name, value)` pairs suitable for passing
    /// to [`eval_expr`](Self::eval_expr).
    pub fn collect_vars(&self) -> Vec<(String, NonFunctionValue)> {
        let mut result = IndexMap::new();
        for scope in &self.scopes {
            for (name, value) in scope {
                result.insert(name.clone(), value.clone());
            }
        }
        result.into_iter().collect()
    }
}

impl Default for CanonicalContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Context<NonFunctionValue> for CanonicalContext {
    fn define(&mut self, name: &str, value: NonFunctionValue) {
        self.define(name, value);
    }

    fn get(&self, name: &str) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        self.get(name)
    }

    fn push_scope(&mut self) {
        self.push_scope();
    }

    fn pop_scope(&mut self) -> Option<IndexMap<String, NonFunctionValue>> {
        self.pop_scope()
    }

    fn depth(&self) -> usize {
        self.depth()
    }

    fn call_count(&self) -> usize {
        self.call_count()
    }

    fn increment_call_count(&self) -> Result<(), Error<NonFunctionValue>> {
        self.increment_call_count()
    }

    fn check_depth(&self) -> Result<(), Error<NonFunctionValue>> {
        self.check_depth()
    }

    fn has_eval(&self) -> bool {
        self.has_eval()
    }

    fn has_async_eval(&self) -> bool {
        self.has_async_eval()
    }

    fn set_eval(&mut self, eval: Box<dyn Eval<NonFunctionValue>>) {
        self.set_eval(eval);
    }

    fn set_async_eval(&mut self, eval: Box<dyn AsyncEval<NonFunctionValue>>) {
        self.set_async_eval(eval);
    }

    fn eval_expr(
        &self,
        expr: &str,
        vars: &[(String, NonFunctionValue)],
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        self.eval_expr(expr, vars)
    }

    fn eval_expr_async<'a>(
        &'a self,
        expr: &'a str,
        vars: &'a [(String, NonFunctionValue)],
    ) -> Pin<Box<dyn Future<Output = Result<NonFunctionValue, Error<NonFunctionValue>>> + 'a>> {
        self.eval_expr_async(expr, vars)
    }

    fn set_max_depth(&mut self, max: usize) {
        self.set_max_depth(max);
    }

    fn set_max_calls(&mut self, max: usize) {
        self.set_max_calls(max);
    }

    fn max_depth(&self) -> Option<usize> {
        self.max_depth()
    }

    fn max_calls(&self) -> Option<usize> {
        self.max_calls()
    }

    fn collect_vars(&self) -> Vec<(String, NonFunctionValue)> {
        self.collect_vars()
    }
}
