use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Instant;

use ac_runtime::{Context, Error, Function, NonFunctionValue, Operation};
use serde_json;

/// Caches results of a function by the JSON representation of its args.
pub struct Memoize<F> {
    inner: F,
    cache: RefCell<HashMap<String, NonFunctionValue>>}

impl<F> Memoize<F> {
    /// Wraps a function with a per-args cache.
    pub fn new(inner: F) -> Self {
        Memoize {
            inner,
            cache: RefCell::new(HashMap::new())}
    }
}

impl<F> Function for Memoize<F>
where
    F: Function + 'static,
{
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let key = args_to_key(args);

        if let Some(cached) = self.cache.borrow().get(&key) {
            return Ok(cached.clone());
        }

        let result = self.inner.call(args, ctx);

        if let Ok(value) = &result {
            self.cache.borrow_mut().insert(key, value.clone());
        }

        result
    }
}

fn args_to_key(args: &[NonFunctionValue]) -> String {
    let values: Vec<&serde_json::Value> = args.iter().map(|v| v.inner()).collect();
    serde_json::to_string(&values).unwrap_or_default()
}

/// Creates a memoized function from another function.
pub fn memoize<F>(inner: F) -> Box<dyn Function>
where
    F: Function + 'static,
{
    Box::new(Memoize::new(inner))
}

/// Transforms arguments before passing them to the inner function.
pub struct MapArgs<F, T> {
    inner: F,
    transform: T}

impl<F, T> MapArgs<F, T> {
    /// Wraps a function, applying a transform to its arguments.
    pub fn new(inner: F, transform: T) -> Self {
        MapArgs { inner, transform }
    }
}

impl<F, T> Function for MapArgs<F, T>
where
    F: Function + 'static,
    T: Fn(&[NonFunctionValue]) -> Vec<NonFunctionValue> + 'static,
{
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let mapped = (self.transform)(args);
        self.inner.call(&mapped, ctx)
    }
}

/// Creates a function that transforms its arguments before evaluation.
pub fn map_args<F, T>(inner: F, transform: T) -> Box<dyn Function>
where
    F: Function + 'static,
    T: Fn(&[NonFunctionValue]) -> Vec<NonFunctionValue> + 'static,
{
    Box::new(MapArgs::new(inner, transform))
}

/// Transforms the result of a function.
pub struct MapResult<F, T> {
    inner: F,
    transform: T}

impl<F, T> MapResult<F, T> {
    /// Wraps a function, applying a transform to its result.
    pub fn new(inner: F, transform: T) -> Self {
        MapResult { inner, transform }
    }
}

impl<F, T> Function for MapResult<F, T>
where
    F: Function + 'static,
    T: Fn(Result<NonFunctionValue, Error>) -> Result<NonFunctionValue, Error> + 'static,
{
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        (self.transform)(self.inner.call(args, ctx))
    }
}

/// Creates a function that transforms its result.
pub fn map_result<F, T>(inner: F, transform: T) -> Box<dyn Function>
where
    F: Function + 'static,
    T: Fn(Result<NonFunctionValue, Error>) -> Result<NonFunctionValue, Error> + 'static,
{
    Box::new(MapResult::new(inner, transform))
}

/// Catches errors from a function and runs a handler to recover.
pub struct Catch<F, H> {
    inner: F,
    handler: H}

impl<F, H> Catch<F, H> {
    /// Wraps a function with an error handler.
    pub fn new(inner: F, handler: H) -> Self {
        Catch { inner, handler }
    }
}

impl<F, H> Function for Catch<F, H>
where
    F: Function + 'static,
    H: Fn(Error) -> Result<NonFunctionValue, Error> + 'static,
{
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        self.inner.call(args, ctx).or_else(|e| (self.handler)(e))
    }
}

/// Creates a function that catches errors and applies a handler.
pub fn catch<F, H>(inner: F, handler: H) -> Box<dyn Function>
where
    F: Function + 'static,
    H: Fn(Error) -> Result<NonFunctionValue, Error> + 'static,
{
    Box::new(Catch::new(inner, handler))
}

/// Pipes the result of one function into a second function.
pub struct Pipe<F, G> {
    first: F,
    second: G}

impl<F, G> Pipe<F, G> {
    /// Pipes `first` into `second`.
    pub fn new(first: F, second: G) -> Self {
        Pipe { first, second }
    }
}

impl<F, G> Function for Pipe<F, G>
where
    F: Function + 'static,
    G: Function + 'static,
{
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let value = self.first.call(args, ctx)?;
        let v = vec![value];
        self.second.call(&v, ctx)
    }
}

/// Creates a function that pipes `first` into `second`.
pub fn pipe<F, G>(first: F, second: G) -> Box<dyn Function>
where
    F: Function + 'static,
    G: Function + 'static,
{
    Box::new(Pipe::new(first, second))
}

/// Logs arguments and results of a function.
pub struct Logged<F, L> {
    inner: F,
    logger: L}

impl<F, L> Logged<F, L> {
    /// Wraps a function with a logger.
    pub fn new(inner: F, logger: L) -> Self {
        Logged { inner, logger }
    }
}

impl<F, L> Function for Logged<F, L>
where
    F: Function + 'static,
    L: Fn(&[NonFunctionValue], &Result<NonFunctionValue, Error>) + 'static,
{
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let result = self.inner.call(args, ctx);
        (self.logger)(args, &result);
        result
    }
}

/// Creates a function that logs its calls and results.
pub fn logged<F, L>(inner: F, logger: L) -> Box<dyn Function>
where
    F: Function + 'static,
    L: Fn(&[NonFunctionValue], &Result<NonFunctionValue, Error>) + 'static,
{
    Box::new(Logged::new(inner, logger))
}

/// Times a function and reports the duration.
pub struct Timed<F, L> {
    inner: F,
    label: String,
    logger: L}

impl<F, L> Timed<F, L> {
    /// Wraps a function with a timer.
    pub fn new(inner: F, label: impl Into<String>, logger: L) -> Self {
        Timed {
            inner,
            label: label.into(),
            logger}
    }
}

impl<F, L> Function for Timed<F, L>
where
    F: Function + 'static,
    L: Fn(&str, &std::time::Duration, &Result<NonFunctionValue, Error>) + 'static,
{
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let start = Instant::now();
        let result = self.inner.call(args, ctx);
        let duration = start.elapsed();
        (self.logger)(&self.label, &duration, &result);
        result
    }
}

/// Creates a function that times its calls.
pub fn timed<F, L>(
    inner: F,
    label: impl Into<String>,
    logger: L,
) -> Box<dyn Function>
where
    F: Function + 'static,
    L: Fn(&str, &std::time::Duration, &Result<NonFunctionValue, Error>) + 'static,
{
    Box::new(Timed::new(inner, label, logger))
}

/// Catches operation errors and recovers with a handler.
pub struct CatchOp<O, H> {
    inner: O,
    handler: H}

impl<O, H> CatchOp<O, H> {
    /// Wraps an operation with an error handler.
    pub fn new(inner: O, handler: H) -> Self {
        CatchOp { inner, handler }
    }
}

impl<O, H> Operation for CatchOp<O, H>
where
    O: Operation + 'static,
    H: Fn(Error) -> Result<NonFunctionValue, Error> + 'static,
{
    fn signature(&self) -> ac_runtime::OperationSig {
        self.inner.signature()
    }

    fn perform(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        self.inner.perform(args, ctx).or_else(|e| (self.handler)(e))
    }
}

/// Creates an operation that catches errors and applies a handler.
pub fn catch_operation<O, H>(inner: O, handler: H) -> Box<dyn Operation>
where
    O: Operation + 'static,
    H: Fn(Error) -> Result<NonFunctionValue, Error> + 'static,
{
    Box::new(CatchOp::new(inner, handler))
}

/// A function that always returns a constant value.
pub struct Const(NonFunctionValue);

impl Const {
    /// Creates a constant function returning `value`.
    pub fn new(value: NonFunctionValue) -> Self {
        Const(value)
    }
}

impl Function for Const {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        Ok(self.0.clone())
    }
}

/// Creates a function that always returns `value`.
pub fn r#const(value: NonFunctionValue) -> Box<dyn Function> {
    Box::new(Const::new(value))
}

/// A function that returns its first argument unchanged.
pub struct Identity;

impl Function for Identity {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        Ok(args.first().cloned().unwrap_or_default())
    }
}

/// Creates the identity function.
pub fn identity() -> Box<dyn Function> {
    Box::new(Identity)
}

/// A function that partially applies the first `n` arguments to `inner`.
pub struct Partial<F> {
    inner: F,
    bound: Vec<NonFunctionValue>,
}

impl<F> Partial<F> {
    /// Binds `bound` to the front of every call to `inner`.
    pub fn new(inner: F, bound: Vec<NonFunctionValue>) -> Self {
        Partial { inner, bound }
    }
}

impl<F> Function for Partial<F>
where
    F: Function + 'static,
{
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let mut full = self.bound.clone();
        full.extend_from_slice(args);
        self.inner.call(&full, ctx)
    }
}

/// Creates a function with `bound` arguments prepended to every call.
pub fn partial<F>(inner: F, bound: Vec<NonFunctionValue>) -> Box<dyn Function>
where
    F: Function + 'static,
{
    Box::new(Partial::new(inner, bound))
}

/// A function that conditionally evaluates one of two lazy branches.
pub struct If<C, T, E> {
    condition: C,
    then_branch: T,
    else_branch: E,
}

impl<C, T, E> If<C, T, E> {
    /// Creates a conditional function.
    pub fn new(condition: C, then_branch: T, else_branch: E) -> Self {
        If {
            condition,
            then_branch,
            else_branch,
        }
    }
}

impl<C, T, E> Function for If<C, T, E>
where
    C: Function + 'static,
    T: Function + 'static,
    E: Function + 'static,
{
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let cond = self.condition.call(args, ctx)?;
        if cond.as_bool().unwrap_or(false) {
            self.then_branch.call(args, ctx)
        } else {
            self.else_branch.call(args, ctx)
        }
    }
}

/// Creates a conditional function.
pub fn r#if<C, T, E>(
    condition: C,
    then_branch: T,
    else_branch: E,
) -> Box<dyn Function>
where
    C: Function + 'static,
    T: Function + 'static,
    E: Function + 'static,
{
    Box::new(If::new(condition, then_branch, else_branch))
}
