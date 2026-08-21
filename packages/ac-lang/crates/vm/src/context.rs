use crate::AsyncEval;
use crate::Error;
use crate::Eval;
use ac_runtime::NonFunctionValue;
use indexmap::IndexMap;
use std::future::Future;
use std::pin::Pin;

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
pub struct Context {
    scopes: Vec<IndexMap<String, NonFunctionValue>>,
    eval: Option<Box<dyn Eval>>,
    async_eval: Option<Box<dyn AsyncEval>>,
}

impl Context {
    /// Creates a new context with a single empty global scope.
    pub fn new() -> Self {
        Context {
            scopes: vec![IndexMap::new()],
            eval: None,
            async_eval: None,
        }
    }

    /// Sets the [`Eval`] callback for this context.
    ///
    /// When set, host functions can call [`Context::eval_expr`] to
    /// evaluate expression strings with variable bindings. This enables
    /// higher-order functions like `fp.map`, `fp.filter`, `fp.reduce`.
    pub fn set_eval(&mut self, eval: Box<dyn Eval>) {
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
    ) -> Result<NonFunctionValue, Error> {
        self.eval
            .as_ref()
            .ok_or(Error::PrimitiveError("no eval callback set".to_string()))?
            .eval_expr(expr, vars)
    }

    /// Sets the [`AsyncEval`] callback for this context.
    ///
    /// When set, async host functions can call [`Context::eval_expr_async`]
    /// to evaluate expression strings with variable bindings asynchronously.
    pub fn set_async_eval(&mut self, eval: Box<dyn AsyncEval>) {
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
    ) -> Pin<Box<dyn Future<Output = Result<NonFunctionValue, Error>> + 'a>> {
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
    pub fn get(&self, name: &str) -> Result<NonFunctionValue, Error> {
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

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
