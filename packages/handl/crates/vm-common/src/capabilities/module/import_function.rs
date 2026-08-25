use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use handl_module_resolver_core::ModuleResolver;
use handl_vm::{Context, Error, Function};
use handl_vm_values_json::NonFunctionValue;

/// A [`Function`] that resolves module imports via a [`ModuleResolver`].
///
/// When called with a string path argument, it delegates to the
/// resolver and returns the imported module's data as a
/// [`NonFunctionValue`]. Results are cached — subsequent imports
/// of the same path return the cached value without re-resolving.
///
/// The function is registered under [`IMPORT_FUNCTION`] (`"import"`
/// by convention). A runtime host may use a different name or provide
/// module loading through a different mechanism entirely.
///
/// # Interior mutability
///
/// The `call` method takes `&self`. The cache uses a [`Mutex`] for
/// interior mutability, making `ImportFunction` `Send + `Sync`.
pub struct ImportFunction {
    resolver: Box<dyn ModuleResolver>,
    base_path: Option<PathBuf>,
    cache: Mutex<HashMap<String, NonFunctionValue>>,
}

impl ImportFunction {
    /// Creates a new import function with the given resolver and no
    /// base path.
    ///
    /// Relative import paths will be resolved without a base —
    /// the resolver decides what to do with a `None` base.
    pub fn new(resolver: Box<dyn ModuleResolver>) -> Self {
        ImportFunction {
            resolver,
            base_path: None,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Sets the base path for relative import resolution.
    ///
    /// The base path is the directory of the importing document.
    /// Relative paths in `import()` calls are resolved against it.
    pub fn with_base_path(mut self, base_path: PathBuf) -> Self {
        self.base_path = Some(base_path);
        self
    }
}

impl Function<NonFunctionValue> for ImportFunction {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let path_value = args.first().ok_or_else(|| Error::ArgumentCount {
            expected: 1,
            got: args.len(),
        })?;

        let path = path_value.as_str().ok_or_else(|| Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })?;

        let cache_key = match &self.base_path {
            Some(base) => base.join(path).to_string_lossy().to_string(),
            None => path.to_string(),
        };

        {
            let cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        let module = self
            .resolver
            .resolve(path, self.base_path.as_deref())
            .ok_or_else(|| Error::PrimitiveError(format!("import: failed to resolve '{path}'")))?;

        let data = NonFunctionValue::new(module.data);

        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(cache_key, data.clone());
        }

        Ok(data)
    }
}
