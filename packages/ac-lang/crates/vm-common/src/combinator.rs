use ac_vm::{AsyncFunction, Function, GenericFunction, Host, LazyFunction, Operation};

/// A host that queries a list of child hosts in order and returns the
/// first match found.
pub struct ChainHost(Vec<Box<dyn Host>>);

impl ChainHost {
    /// Creates an empty chain host.
    pub fn new() -> Self {
        ChainHost(Vec::new())
    }

    /// Adds a child host to the end of the chain.
    pub fn push(mut self, host: Box<dyn Host>) -> Self {
        self.0.push(host);
        self
    }

    /// Adds a child host by value.
    pub fn with<H: Host + 'static>(mut self, host: H) -> Self {
        self.0.push(Box::new(host));
        self
    }
}

impl Default for ChainHost {
    fn default() -> Self {
        Self::new()
    }
}

impl Host for ChainHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        self.0.iter().find_map(|h| h.function(name))
    }

    fn async_function(&self, name: &str) -> Option<&dyn AsyncFunction> {
        self.0.iter().find_map(|h| h.async_function(name))
    }

    fn operation(&self, name: &str) -> Option<&dyn Operation> {
        self.0.iter().find_map(|h| h.operation(name))
    }

    fn lazy_function(&self, name: &str) -> Option<&dyn LazyFunction> {
        self.0.iter().find_map(|h| h.lazy_function(name))
    }

    fn generic_function(&self, name: &str) -> Option<&dyn GenericFunction> {
        self.0.iter().find_map(|h| h.generic_function(name))
    }
}

/// A host that wraps another host and filters out names matching a
/// predicate. Useful for forbidding specific functions in a sandbox.
pub struct FilterHost {
    inner: Box<dyn Host>,
    predicate: Box<dyn Fn(&str) -> bool + 'static>}

impl FilterHost {
    /// Creates a host that forwards only names allowed by `predicate`.
    pub fn new(inner: Box<dyn Host>, predicate: impl Fn(&str) -> bool + 'static) -> Self {
        FilterHost {
            inner,
            predicate: Box::new(predicate)}
    }

    /// Creates a host that blocks the given names.
    pub fn block(inner: Box<dyn Host>, names: &[&str]) -> Self {
        let set: std::collections::HashSet<String> =
            names.iter().map(|&s| s.to_string()).collect();
        FilterHost::new(inner, move |name| !set.contains(name))
    }
}

impl Host for FilterHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        if !(self.predicate)(name) {
            return None;
        }
        self.inner.function(name)
    }

    fn async_function(&self, name: &str) -> Option<&dyn AsyncFunction> {
        if !(self.predicate)(name) {
            return None;
        }
        self.inner.async_function(name)
    }

    fn operation(&self, name: &str) -> Option<&dyn Operation> {
        if !(self.predicate)(name) {
            return None;
        }
        self.inner.operation(name)
    }

    fn lazy_function(&self, name: &str) -> Option<&dyn LazyFunction> {
        if !(self.predicate)(name) {
            return None;
        }
        self.inner.lazy_function(name)
    }

    fn generic_function(&self, name: &str) -> Option<&dyn GenericFunction> {
        if !(self.predicate)(name) {
            return None;
        }
        self.inner.generic_function(name)
    }
}

/// A host that wraps another host and logs every lookup.
pub struct SpyHost {
    inner: Box<dyn Host>,
    lookups: std::cell::RefCell<Vec<String>>}

impl SpyHost {
    /// Creates a host that records every name it is asked to resolve.
    pub fn new(inner: Box<dyn Host>) -> Self {
        SpyHost {
            inner,
            lookups: std::cell::RefCell::new(Vec::new())}
    }

    /// Returns the recorded lookups.
    pub fn lookups(&self) -> Vec<String> {
        self.lookups.borrow().clone()
    }
}

impl Host for SpyHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        self.lookups.borrow_mut().push(name.to_string());
        self.inner.function(name)
    }

    fn async_function(&self, name: &str) -> Option<&dyn AsyncFunction> {
        self.lookups.borrow_mut().push(name.to_string());
        self.inner.async_function(name)
    }

    fn operation(&self, name: &str) -> Option<&dyn Operation> {
        self.lookups.borrow_mut().push(name.to_string());
        self.inner.operation(name)
    }

    fn lazy_function(&self, name: &str) -> Option<&dyn LazyFunction> {
        self.lookups.borrow_mut().push(name.to_string());
        self.inner.lazy_function(name)
    }

    fn generic_function(&self, name: &str) -> Option<&dyn GenericFunction> {
        self.lookups.borrow_mut().push(name.to_string());
        self.inner.generic_function(name)
    }
}
