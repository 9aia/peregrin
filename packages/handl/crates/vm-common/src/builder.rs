use handl_vm::{
    AsyncFunction, Context, Error, Function, GenericFunction, LazyFunction, Operation, OperationSig,
};
use handl_vm_values_json::NonFunctionValue;
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

impl<F> Function<NonFunctionValue> for FnFunction<F>
where
    F: Fn(
            &[NonFunctionValue],
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>
        + 'static,
{
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        (self.0)(args, ctx)
    }
}

/// Creates a synchronous function from a closure.
pub fn fn_function<F>(f: F) -> Box<dyn Function<NonFunctionValue>>
where
    F: Fn(
            &[NonFunctionValue],
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>
        + 'static,
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

impl<S, P> Operation<NonFunctionValue> for FnOperation<S, P>
where
    S: Fn() -> OperationSig<NonFunctionValue> + 'static,
    P: Fn(
            &[NonFunctionValue],
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>
        + 'static,
{
    fn signature(&self) -> OperationSig<NonFunctionValue> {
        (self.signature)()
    }

    fn perform(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        (self.perform)(args, ctx)
    }
}

/// Creates an operation from a signature closure and a perform closure.
pub fn fn_operation<S, P>(signature: S, perform: P) -> Box<dyn Operation<NonFunctionValue>>
where
    S: Fn() -> OperationSig<NonFunctionValue> + 'static,
    P: Fn(
            &[NonFunctionValue],
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>
        + 'static,
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

impl<F> LazyFunction<NonFunctionValue> for FnLazy<F>
where
    F: Fn(
            &[handl_vm::Expr<NonFunctionValue>],
            &mut dyn Context<NonFunctionValue>,
            &mut dyn FnMut(
                &handl_vm::Expr<NonFunctionValue>,
                &mut dyn Context<NonFunctionValue>,
            ) -> Result<NonFunctionValue, Error<NonFunctionValue>>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>
        + 'static,
{
    fn call(
        &self,
        args: &[handl_vm::Expr<NonFunctionValue>],
        ctx: &mut dyn Context<NonFunctionValue>,
        evaluate: &mut dyn FnMut(
            &handl_vm::Expr<NonFunctionValue>,
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        (self.0)(args, ctx, evaluate)
    }
}

/// Creates a lazy function from a closure.
pub fn fn_lazy<F>(f: F) -> Box<dyn LazyFunction<NonFunctionValue>>
where
    F: Fn(
            &[handl_vm::Expr<NonFunctionValue>],
            &mut dyn Context<NonFunctionValue>,
            &mut dyn FnMut(
                &handl_vm::Expr<NonFunctionValue>,
                &mut dyn Context<NonFunctionValue>,
            ) -> Result<NonFunctionValue, Error<NonFunctionValue>>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>
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

impl<F> Function<NonFunctionValue> for FnGeneric<F>
where
    F: Fn(
            &[NonFunctionValue],
            &[String],
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>
        + 'static,
{
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        (self.0)(args, &[], ctx)
    }
}

impl<F> GenericFunction<NonFunctionValue> for FnGeneric<F>
where
    F: Fn(
            &[NonFunctionValue],
            &[String],
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>
        + 'static,
{
    fn call_with_types(
        &self,
        args: &[NonFunctionValue],
        type_args: &[String],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        (self.0)(args, type_args, ctx)
    }
}

/// Creates a generic function from a closure.
pub fn fn_generic<F>(f: F) -> Box<dyn GenericFunction<NonFunctionValue>>
where
    F: Fn(
            &[NonFunctionValue],
            &[String],
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>
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

impl<F> AsyncFunction<NonFunctionValue> for FnAsync<F>
where
    F: for<'a> Fn(
            &'a [NonFunctionValue],
            &'a mut (dyn Context<NonFunctionValue> + 'a),
        ) -> Pin<
            Box<dyn Future<Output = Result<NonFunctionValue, Error<NonFunctionValue>>> + 'a>,
        > + 'static,
{
    fn call<'a>(
        &'a self,
        args: &'a [NonFunctionValue],
        ctx: &'a mut (dyn Context<NonFunctionValue> + 'a),
    ) -> Pin<Box<dyn Future<Output = Result<NonFunctionValue, Error<NonFunctionValue>>> + 'a>> {
        (self.0)(args, ctx)
    }
}

/// Creates an async function from a higher-ranked closure.
pub fn fn_async<F>(f: F) -> Box<dyn AsyncFunction<NonFunctionValue>>
where
    F: for<'a> Fn(
            &'a [NonFunctionValue],
            &'a mut (dyn Context<NonFunctionValue> + 'a),
        ) -> Pin<
            Box<dyn Future<Output = Result<NonFunctionValue, Error<NonFunctionValue>>> + 'a>,
        > + 'static,
{
    Box::new(FnAsync::new(f))
}
