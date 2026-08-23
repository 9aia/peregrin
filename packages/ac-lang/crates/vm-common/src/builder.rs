use ac_runtime::{
    AsyncFunction, Context, Error, Function, GenericFunction, LazyFunction, NonFunctionValue,
    Operation, OperationSig,
};
use std::future::Future;
use std::pin::Pin;

/// A function backed by a closure.
pub struct FnFunction<F>(F);

impl<F> FnFunction<F> {
    /// Creates a new function from a closure.
    pub fn new(f: F) -> Self {
        FnFunction(f)
    }
}

impl<F> Function for FnFunction<F>
where
    F: Fn(&[NonFunctionValue], &mut dyn Context) -> Result<NonFunctionValue, Error> + 'static,
{
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        (self.0)(args, ctx)
    }
}

/// Creates a synchronous function from a closure.
pub fn fn_function<F>(f: F) -> Box<dyn Function>
where
    F: Fn(&[NonFunctionValue], &mut dyn Context) -> Result<NonFunctionValue, Error> + 'static,
{
    Box::new(FnFunction::new(f))
}

/// An operation backed by closures for its signature and behavior.
pub struct FnOperation<S, P> {
    signature: S,
    perform: P,
}

impl<S, P> FnOperation<S, P> {
    /// Creates a new operation from a signature closure and a perform closure.
    pub fn new(signature: S, perform: P) -> Self {
        FnOperation { signature, perform }
    }
}

impl<S, P> Operation for FnOperation<S, P>
where
    S: Fn() -> OperationSig + 'static,
    P: Fn(&[NonFunctionValue], &mut dyn Context) -> Result<NonFunctionValue, Error> + 'static,
{
    fn signature(&self) -> OperationSig {
        (self.signature)()
    }

    fn perform(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        (self.perform)(args, ctx)
    }
}

/// Creates an operation from a signature closure and a perform closure.
pub fn fn_operation<S, P>(signature: S, perform: P) -> Box<dyn Operation>
where
    S: Fn() -> OperationSig + 'static,
    P: Fn(&[NonFunctionValue], &mut dyn Context) -> Result<NonFunctionValue, Error> + 'static,
{
    Box::new(FnOperation::new(signature, perform))
}

/// A lazy function backed by a closure.
pub struct FnLazy<F>(F);

impl<F> FnLazy<F> {
    /// Creates a new lazy function from a closure.
    pub fn new(f: F) -> Self {
        FnLazy(f)
    }
}

impl<F> LazyFunction for FnLazy<F>
where
    F: Fn(
            &[ac_runtime::Expr<NonFunctionValue>],
            &mut dyn Context,
            &mut dyn FnMut(
                &ac_runtime::Expr<NonFunctionValue>,
                &mut dyn Context,
            ) -> Result<NonFunctionValue, Error>,
        ) -> Result<NonFunctionValue, Error>
        + 'static,
{
    fn call(
        &self,
        args: &[ac_runtime::Expr<NonFunctionValue>],
        ctx: &mut dyn Context,
        evaluate: &mut dyn FnMut(
            &ac_runtime::Expr<NonFunctionValue>,
            &mut dyn Context,
        ) -> Result<NonFunctionValue, Error>,
    ) -> Result<NonFunctionValue, Error> {
        (self.0)(args, ctx, evaluate)
    }
}

/// Creates a lazy function from a closure.
pub fn fn_lazy<F>(f: F) -> Box<dyn LazyFunction>
where
    F: Fn(
            &[ac_runtime::Expr<NonFunctionValue>],
            &mut dyn Context,
            &mut dyn FnMut(
                &ac_runtime::Expr<NonFunctionValue>,
                &mut dyn Context,
            ) -> Result<NonFunctionValue, Error>,
        ) -> Result<NonFunctionValue, Error>
        + 'static,
{
    Box::new(FnLazy::new(f))
}

/// A generic function backed by a closure.
pub struct FnGeneric<F>(F);

impl<F> FnGeneric<F> {
    /// Creates a new generic function from a closure.
    pub fn new(f: F) -> Self {
        FnGeneric(f)
    }
}

impl<F> Function for FnGeneric<F>
where
    F: Fn(&[NonFunctionValue], &[String], &mut dyn Context) -> Result<NonFunctionValue, Error>
        + 'static,
{
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        (self.0)(args, &[], ctx)
    }
}

impl<F> GenericFunction for FnGeneric<F>
where
    F: Fn(&[NonFunctionValue], &[String], &mut dyn Context) -> Result<NonFunctionValue, Error>
        + 'static,
{
    fn call_with_types(
        &self,
        args: &[NonFunctionValue],
        type_args: &[String],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        (self.0)(args, type_args, ctx)
    }
}

/// Creates a generic function from a closure.
pub fn fn_generic<F>(f: F) -> Box<dyn GenericFunction>
where
    F: Fn(&[NonFunctionValue], &[String], &mut dyn Context) -> Result<NonFunctionValue, Error>
        + 'static,
{
    Box::new(FnGeneric::new(f))
}

/// An async function backed by a closure.
pub struct FnAsync<F>(F);

impl<F> FnAsync<F> {
    /// Creates a new async function from a closure.
    pub fn new(f: F) -> Self {
        FnAsync(f)
    }
}

impl<F> AsyncFunction for FnAsync<F>
where
    F: for<'a> Fn(
            &'a [NonFunctionValue],
            &'a mut (dyn Context + 'a),
        ) -> Pin<Box<dyn Future<Output = Result<NonFunctionValue, Error>> + 'a>>
        + 'static,
{
    fn call<'a>(
        &'a self,
        args: &'a [NonFunctionValue],
        ctx: &'a mut (dyn Context + 'a),
    ) -> Pin<Box<dyn Future<Output = Result<NonFunctionValue, Error>> + 'a>> {
        (self.0)(args, ctx)
    }
}

/// Creates an async function from a higher-ranked closure.
pub fn fn_async<F>(f: F) -> Box<dyn AsyncFunction>
where
    F: for<'a> Fn(
            &'a [NonFunctionValue],
            &'a mut (dyn Context + 'a),
        ) -> Pin<Box<dyn Future<Output = Result<NonFunctionValue, Error>> + 'a>>
        + 'static,
{
    Box::new(FnAsync::new(f))
}
