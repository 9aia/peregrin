use ac_runtime::{AsyncFunction, Function, GenericFunction, Host, LazyFunction, Operation};

/// A host that rewrites lookup names before forwarding them.
///
/// Useful for namespacing or versioning: `"math.add"` → `"add"`.
pub struct MapHost {
    inner: Box<dyn Host>,
    mapper: Box<dyn Fn(&str) -> String + 'static>,
}

impl MapHost {
    /// Creates a host that maps names through `mapper`.
    pub fn new(inner: Box<dyn Host>, mapper: impl Fn(&str) -> String + 'static) -> Self {
        MapHost {
            inner,
            mapper: Box::new(mapper),
        }
    }

    /// Convenience constructor that prepends a fixed prefix.
    pub fn with_prefix(inner: Box<dyn Host>, prefix: impl Into<String>) -> Self {
        let prefix = prefix.into();
        MapHost::new(inner, move |name| format!("{prefix}{name}"))
    }

    /// Convenience constructor that strips a fixed prefix.
    pub fn strip_prefix(inner: Box<dyn Host>, prefix: impl Into<String>) -> Self {
        let prefix = prefix.into();
        MapHost::new(inner, move |name| {
            name.strip_prefix(&prefix).unwrap_or(name).to_string()
        })
    }

    fn map(&self, name: &str) -> String {
        (self.mapper)(name)
    }
}

impl Host for MapHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        self.inner.function(&self.map(name))
    }

    fn async_function(&self, name: &str) -> Option<&dyn AsyncFunction> {
        self.inner.async_function(&self.map(name))
    }

    fn operation(&self, name: &str) -> Option<&dyn Operation> {
        self.inner.operation(&self.map(name))
    }

    fn lazy_function(&self, name: &str) -> Option<&dyn LazyFunction> {
        self.inner.lazy_function(&self.map(name))
    }

    fn generic_function(&self, name: &str) -> Option<&dyn GenericFunction> {
        self.inner.generic_function(&self.map(name))
    }
}

/// A host that logs every lookup it forwards.
pub struct LoggingHost {
    inner: Box<dyn Host>,
    log: Box<dyn Fn(&HostEvent<'_>) + 'static>,
}

/// What was looked up on a host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostEventKind {
    Function,
    AsyncFunction,
    Operation,
    LazyFunction,
    GenericFunction,
}

/// A host-lookup event.
#[derive(Debug)]
pub struct HostEvent<'a> {
    pub kind: HostEventKind,
    pub name: &'a str,
    pub found: bool,
}

impl LoggingHost {
    /// Creates a host that logs every lookup to `logger`.
    pub fn new(inner: Box<dyn Host>, logger: impl Fn(&HostEvent<'_>) + 'static) -> Self {
        LoggingHost {
            inner,
            log: Box::new(logger),
        }
    }

    /// Creates a host that prints every lookup to stderr.
    pub fn eprint(inner: Box<dyn Host>) -> Self {
        LoggingHost::new(inner, |event| {
            eprintln!(
                "[host] {:?} {} found={}",
                event.kind, event.name, event.found
            );
        })
    }
}

impl Host for LoggingHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        let result = self.inner.function(name);
        (self.log)(&HostEvent {
            kind: HostEventKind::Function,
            name,
            found: result.is_some(),
        });
        result
    }

    fn async_function(&self, name: &str) -> Option<&dyn AsyncFunction> {
        let result = self.inner.async_function(name);
        (self.log)(&HostEvent {
            kind: HostEventKind::AsyncFunction,
            name,
            found: result.is_some(),
        });
        result
    }

    fn operation(&self, name: &str) -> Option<&dyn Operation> {
        let result = self.inner.operation(name);
        (self.log)(&HostEvent {
            kind: HostEventKind::Operation,
            name,
            found: result.is_some(),
        });
        result
    }

    fn lazy_function(&self, name: &str) -> Option<&dyn LazyFunction> {
        let result = self.inner.lazy_function(name);
        (self.log)(&HostEvent {
            kind: HostEventKind::LazyFunction,
            name,
            found: result.is_some(),
        });
        result
    }

    fn generic_function(&self, name: &str) -> Option<&dyn GenericFunction> {
        let result = self.inner.generic_function(name);
        (self.log)(&HostEvent {
            kind: HostEventKind::GenericFunction,
            name,
            found: result.is_some(),
        });
        result
    }
}

/// A host that only forwards a fixed set of allowed names.
pub struct AllowHost {
    inner: Box<dyn Host>,
    allowed: std::collections::HashSet<String>,
}

impl AllowHost {
    /// Creates a host that only resolves names in `allowed`.
    pub fn new(inner: Box<dyn Host>, allowed: impl IntoIterator<Item = impl Into<String>>) -> Self {
        AllowHost {
            inner,
            allowed: allowed.into_iter().map(|s| s.into()).collect(),
        }
    }
}

impl Host for AllowHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        if self.allowed.contains(name) {
            self.inner.function(name)
        } else {
            None
        }
    }

    fn async_function(&self, name: &str) -> Option<&dyn AsyncFunction> {
        if self.allowed.contains(name) {
            self.inner.async_function(name)
        } else {
            None
        }
    }

    fn operation(&self, name: &str) -> Option<&dyn Operation> {
        if self.allowed.contains(name) {
            self.inner.operation(name)
        } else {
            None
        }
    }

    fn lazy_function(&self, name: &str) -> Option<&dyn LazyFunction> {
        if self.allowed.contains(name) {
            self.inner.lazy_function(name)
        } else {
            None
        }
    }

    fn generic_function(&self, name: &str) -> Option<&dyn GenericFunction> {
        if self.allowed.contains(name) {
            self.inner.generic_function(name)
        } else {
            None
        }
    }
}

/// A host that blocks a fixed set of names.
pub struct DenyHost {
    inner: Box<dyn Host>,
    denied: std::collections::HashSet<String>,
}

impl DenyHost {
    /// Creates a host that denies names in `denied`.
    pub fn new(inner: Box<dyn Host>, denied: impl IntoIterator<Item = impl Into<String>>) -> Self {
        DenyHost {
            inner,
            denied: denied.into_iter().map(|s| s.into()).collect(),
        }
    }
}

impl Host for DenyHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        if self.denied.contains(name) {
            None
        } else {
            self.inner.function(name)
        }
    }

    fn async_function(&self, name: &str) -> Option<&dyn AsyncFunction> {
        if self.denied.contains(name) {
            None
        } else {
            self.inner.async_function(name)
        }
    }

    fn operation(&self, name: &str) -> Option<&dyn Operation> {
        if self.denied.contains(name) {
            None
        } else {
            self.inner.operation(name)
        }
    }

    fn lazy_function(&self, name: &str) -> Option<&dyn LazyFunction> {
        if self.denied.contains(name) {
            None
        } else {
            self.inner.lazy_function(name)
        }
    }

    fn generic_function(&self, name: &str) -> Option<&dyn GenericFunction> {
        if self.denied.contains(name) {
            None
        } else {
            self.inner.generic_function(name)
        }
    }
}
