use handl_vm::{AsyncFunction, Function, GenericFunction, Host, LazyFunction, Operation};
use handl_vm_values_json::NonFunctionValue;
use std::collections::HashMap;

/// A host that resolves nothing.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopHost;

impl Host<NonFunctionValue> for NoopHost {
    fn function(&self, _name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        None
    }

    fn async_function(&self, _name: &str) -> Option<&dyn AsyncFunction<NonFunctionValue>> {
        None
    }

    fn operation(&self, _name: &str) -> Option<&dyn Operation<NonFunctionValue>> {
        None
    }

    fn lazy_function(&self, _name: &str) -> Option<&dyn LazyFunction<NonFunctionValue>> {
        None
    }

    fn generic_function(&self, _name: &str) -> Option<&dyn GenericFunction<NonFunctionValue>> {
        None
    }
}

/// A host built from maps of named callables.
#[derive(Default)]
pub struct MockHost {
    functions: HashMap<String, Box<dyn Function<NonFunctionValue>>>,
    async_functions: HashMap<String, Box<dyn AsyncFunction<NonFunctionValue>>>,
    operations: HashMap<String, Box<dyn Operation<NonFunctionValue>>>,
    lazy_functions: HashMap<String, Box<dyn LazyFunction<NonFunctionValue>>>,
    generic_functions: HashMap<String, Box<dyn GenericFunction<NonFunctionValue>>>,
}

impl MockHost {
    /// Creates an empty mock host.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a synchronous function.
    pub fn with_function(
        mut self,
        name: impl Into<String>,
        f: Box<dyn Function<NonFunctionValue>>,
    ) -> Self {
        self.functions.insert(name.into(), f);
        self
    }

    /// Registers an async function.
    pub fn with_async(
        mut self,
        name: impl Into<String>,
        f: Box<dyn AsyncFunction<NonFunctionValue>>,
    ) -> Self {
        self.async_functions.insert(name.into(), f);
        self
    }

    /// Registers an operation.
    pub fn with_operation(
        mut self,
        name: impl Into<String>,
        f: Box<dyn Operation<NonFunctionValue>>,
    ) -> Self {
        self.operations.insert(name.into(), f);
        self
    }

    /// Registers a lazy function.
    pub fn with_lazy(
        mut self,
        name: impl Into<String>,
        f: Box<dyn LazyFunction<NonFunctionValue>>,
    ) -> Self {
        self.lazy_functions.insert(name.into(), f);
        self
    }

    /// Registers a generic function.
    pub fn with_generic(
        mut self,
        name: impl Into<String>,
        f: Box<dyn GenericFunction<NonFunctionValue>>,
    ) -> Self {
        self.generic_functions.insert(name.into(), f);
        self
    }
}

impl Host<NonFunctionValue> for MockHost {
    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        self.functions.get(name).map(|f| f.as_ref())
    }

    fn async_function(&self, name: &str) -> Option<&dyn AsyncFunction<NonFunctionValue>> {
        self.async_functions.get(name).map(|f| f.as_ref())
    }

    fn operation(&self, name: &str) -> Option<&dyn Operation<NonFunctionValue>> {
        self.operations.get(name).map(|f| f.as_ref())
    }

    fn lazy_function(&self, name: &str) -> Option<&dyn LazyFunction<NonFunctionValue>> {
        self.lazy_functions.get(name).map(|f| f.as_ref())
    }

    fn generic_function(&self, name: &str) -> Option<&dyn GenericFunction<NonFunctionValue>> {
        self.generic_functions.get(name).map(|f| f.as_ref())
    }
}

/// A function that records every call it receives.
pub struct CaptureFn<F> {
    inner: F,
    calls: std::cell::RefCell<Vec<Vec<NonFunctionValue>>>,
}

impl<F> CaptureFn<F> {
    /// Wraps a function, capturing each set of arguments.
    pub fn new(inner: F) -> Self {
        CaptureFn {
            inner,
            calls: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Returns the captured argument lists.
    pub fn calls(&self) -> Vec<Vec<NonFunctionValue>> {
        self.calls.borrow().clone()
    }
}

impl<F> Function<NonFunctionValue> for CaptureFn<F>
where
    F: Fn(
            &[NonFunctionValue],
            &mut dyn handl_vm::Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, handl_vm::Error<NonFunctionValue>>
        + 'static,
{
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn handl_vm::Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, handl_vm::Error<NonFunctionValue>> {
        self.calls.borrow_mut().push(args.to_vec());
        (self.inner)(args, ctx)
    }
}
