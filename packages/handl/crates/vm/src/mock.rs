use std::collections::HashMap;

use crate::{AsyncFunction, Function, GenericFunction, Host, LazyFunction, Operation, Value};

/// A host that resolves nothing.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopHost;

impl<V: Value> Host<V> for NoopHost {
    fn function(&self, _name: &str) -> Option<&dyn Function<V>> {
        None
    }

    fn async_function(&self, _name: &str) -> Option<&dyn AsyncFunction<V>> {
        None
    }

    fn operation(&self, _name: &str) -> Option<&dyn Operation<V>> {
        None
    }

    fn lazy_function(&self, _name: &str) -> Option<&dyn LazyFunction<V>> {
        None
    }

    fn generic_function(&self, _name: &str) -> Option<&dyn GenericFunction<V>> {
        None
    }
}

/// A host built from maps of named callables.
pub struct MockHost<V: Value> {
    functions: HashMap<String, Box<dyn Function<V>>>,
    async_functions: HashMap<String, Box<dyn AsyncFunction<V>>>,
    operations: HashMap<String, Box<dyn Operation<V>>>,
    lazy_functions: HashMap<String, Box<dyn LazyFunction<V>>>,
    generic_functions: HashMap<String, Box<dyn GenericFunction<V>>>,
}

impl<V: Value> Default for MockHost<V> {
    fn default() -> Self {
        Self {
            functions: HashMap::new(),
            async_functions: HashMap::new(),
            operations: HashMap::new(),
            lazy_functions: HashMap::new(),
            generic_functions: HashMap::new(),
        }
    }
}

impl<V: Value> MockHost<V> {
    /// Creates an empty mock host.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a synchronous function.
    pub fn with_function(mut self, name: impl Into<String>, f: Box<dyn Function<V>>) -> Self {
        self.functions.insert(name.into(), f);
        self
    }

    /// Registers an async function.
    pub fn with_async(mut self, name: impl Into<String>, f: Box<dyn AsyncFunction<V>>) -> Self {
        self.async_functions.insert(name.into(), f);
        self
    }

    /// Registers an operation.
    pub fn with_operation(mut self, name: impl Into<String>, f: Box<dyn Operation<V>>) -> Self {
        self.operations.insert(name.into(), f);
        self
    }

    /// Registers a lazy function.
    pub fn with_lazy(mut self, name: impl Into<String>, f: Box<dyn LazyFunction<V>>) -> Self {
        self.lazy_functions.insert(name.into(), f);
        self
    }

    /// Registers a generic function.
    pub fn with_generic(mut self, name: impl Into<String>, f: Box<dyn GenericFunction<V>>) -> Self {
        self.generic_functions.insert(name.into(), f);
        self
    }
}

impl<V: Value> Host<V> for MockHost<V> {
    fn function(&self, name: &str) -> Option<&dyn Function<V>> {
        self.functions.get(name).map(|f| f.as_ref())
    }

    fn async_function(&self, name: &str) -> Option<&dyn AsyncFunction<V>> {
        self.async_functions.get(name).map(|f| f.as_ref())
    }

    fn operation(&self, name: &str) -> Option<&dyn Operation<V>> {
        self.operations.get(name).map(|f| f.as_ref())
    }

    fn lazy_function(&self, name: &str) -> Option<&dyn LazyFunction<V>> {
        self.lazy_functions.get(name).map(|f| f.as_ref())
    }

    fn generic_function(&self, name: &str) -> Option<&dyn GenericFunction<V>> {
        self.generic_functions.get(name).map(|f| f.as_ref())
    }
}
