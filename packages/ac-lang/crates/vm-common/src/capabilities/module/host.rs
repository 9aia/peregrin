use std::path::PathBuf;

use ac_module_resolver_core::{IMPORT_FUNCTION, ModuleResolver};
use ac_runtime::{AsyncFunction, Function, Host, LazyFunction, Operation};
use ac_runtime_values::NonFunctionValue;

use super::import_function::ImportFunction;

/// A [`Host`] wrapper that adds runtime `import()` support.
///
/// Delegates all function lookups to the inner host, except for
/// [`IMPORT_FUNCTION`] (`"import"` by convention), which is handled
/// by an [`ImportFunction`] backed by a [`ModuleResolver`].
///
/// This enables programs to call `import("path")` at runtime —
/// not just at type-check time. The resolver determines where modules
/// come from (filesystem, HTTP, git, in-memory, or a chain of these).
///
/// # Example
///
/// ```ignore
/// use ac_runtime_example_1::DefaultHost;
/// use ac_runtime_example_1_resolver::ResolverHost;
/// use ac_module_resolver_virtual::VirtualResolver;
/// use ac_runtime_canonical::{CanonicalContext, Vm, Context};
/// use ac_ast_canonical::Expr;
///
/// let mut resolver = VirtualResolver::new();
/// resolver.add("config.yml", serde_json::json!({
///     "$": { "host": { "type": "string" } },
///     "host": "localhost"
/// }));
///
/// let host = ResolverHost::new(
///     DefaultHost::default(),
///     Box::new(resolver),
/// );
/// let vm = Vm::new(host);
/// let mut ctx = CanonicalContext::new();
///
/// let expr = Expr::call("import", vec![Expr::value("config.yml")]);
/// let result = vm.eval(&expr, &mut ctx).unwrap();
/// ```
pub struct ResolverHost<H: Host<NonFunctionValue>> {
    inner: H,
    import_fn: ImportFunction,
}

impl<H: Host<NonFunctionValue>> ResolverHost<H> {
    /// Creates a new `ResolverHost` wrapping the given inner host.
    ///
    /// The resolver is used to resolve `import()` calls at runtime.
    /// All other function lookups delegate to the inner host.
    pub fn new(inner: H, resolver: Box<dyn ModuleResolver>) -> Self {
        ResolverHost {
            inner,
            import_fn: ImportFunction::new(resolver),
        }
    }

    /// Sets the base path for relative import resolution.
    ///
    /// The base path is the directory of the importing document.
    /// Relative paths in `import()` calls are resolved against it.
    pub fn with_base_path(mut self, base_path: PathBuf) -> Self {
        self.import_fn = self.import_fn.with_base_path(base_path);
        self
    }

    /// Returns a reference to the inner host.
    pub fn inner(&self) -> &H {
        &self.inner
    }
}

impl<H: Host<NonFunctionValue>> Host<NonFunctionValue> for ResolverHost<H> {
    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        if name == IMPORT_FUNCTION {
            return Some(&self.import_fn);
        }
        self.inner.function(name)
    }

    fn async_function(&self, name: &str) -> Option<&dyn AsyncFunction<NonFunctionValue>> {
        self.inner.async_function(name)
    }

    fn operation(&self, name: &str) -> Option<&dyn Operation<NonFunctionValue>> {
        self.inner.operation(name)
    }

    fn lazy_function(&self, name: &str) -> Option<&dyn LazyFunction<NonFunctionValue>> {
        self.inner.lazy_function(name)
    }
}
