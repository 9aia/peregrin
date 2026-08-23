//! VM-backed compiler host for field interpolation and computation.
//!
//! `CompilerHost` wraps [`ac_runtime_example_1::DefaultHost`] and adds `import()`
//! resolution plus user-defined functions stored in the default namespace
//! (`doc.<name>` or `doc.functions.<name>`).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use ac_dialect::Dialect;
use ac_dialect_canonical::AC_DIALECT;
use ac_fields::{self, DEFAULT_NAMESPACE};
use ac_runtime_example_1::DefaultHost;
use ac_module_resolver_core::{ModuleResolver, ResolvedModule};
use ac_runtime::{FunctionValue, NonFunctionValue};
use ac_runtime::{Context, Error, Function, Host};
use serde_json::Value as JsonValue;

/// A host that combines the default AC primitives with compiler-specific
/// behavior: `import()` resolution and user-defined data functions.
#[derive(Clone)]
pub struct CompilerHost {
    default: DefaultHost,
    import: Import,
    data_functions: HashMap<String, DataFunction>}

impl CompilerHost {
    /// Creates a new host from a default host, import resolver, base path,
    /// and the current namespaces.
    pub fn new(
        default: DefaultHost,
        import: Import,
        namespaces: &ac_fields::Namespaces,
    ) -> Self {
        let mut data_functions = HashMap::new();
        if let Some(doc) = namespaces.get(DEFAULT_NAMESPACE) {
            collect_data_functions("", doc, &mut data_functions);
            if let Some(functions) = doc.get("functions").and_then(|v| v.as_object()) {
                for (name, value) in functions {
                    if let Some(fn_val) = FunctionValue::from_json(value) {
                        data_functions.insert(name.clone(), DataFunction::from(&fn_val));
                    }
                }
            }
        }
        CompilerHost {
            default,
            import,
            data_functions}
    }
}

impl Host for CompilerHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        if name == "import" {
            return Some(&self.import);
        }
        if let Some(function) = self.default.function(name) {
            return Some(function);
        }
        self.data_functions.get(name).map(|f| f as &dyn Function)
    }
}

/// A callable wrapper around a function value stored in `doc` or
/// `doc.functions`.
#[derive(Clone)]
struct DataFunction {
    params: Vec<String>,
    body: String,
    return_schema: Option<JsonValue>}

impl DataFunction {
    fn from(value: &FunctionValue) -> Self {
        DataFunction {
            params: value.params.iter().map(|(n, _)| n.clone()).collect(),
            body: value.body.clone(),
            return_schema: value.return_schema.clone()}
    }
}

impl Function for DataFunction {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let vars: Vec<(String, NonFunctionValue)> = self
            .params
            .iter()
            .zip(args.iter())
            .map(|(p, a)| (p.clone(), a.clone()))
            .collect();

        let value = ctx.eval_expr(&self.body, &vars)?;

        if let Some(raw) = &self.return_schema {
            let schema = AC_DIALECT
                .schema_vocabulary()
                .compile(raw)
                .map_err(|e| Error::PrimitiveError(format!("invalid return schema: {e}")))?;
            ac_fields::validate_return(&value, schema.as_ref())
                .map_err(Error::PrimitiveError)?;
        }

        Ok(value)
    }
}

/// Resolves `import(path)` by looking up an external module.
#[derive(Clone)]
pub struct Import {
    base_path: Option<PathBuf>,
    import_cache: Rc<RefCell<HashMap<String, JsonValue>>>,
    resolver: Rc<Box<dyn ModuleResolver>>}

impl Import {
    /// Creates a new `import` host function.
    pub fn new(
        base_path: Option<PathBuf>,
        resolver: Rc<Box<dyn ModuleResolver>>,
        import_cache: Rc<RefCell<HashMap<String, JsonValue>>>,
    ) -> Self {
        Import {
            base_path,
            import_cache,
            resolver}
    }

    /// Helper to build a cache key for an import path.
    pub fn cache_key(&self, path: &str) -> String {
        match &self.base_path {
            Some(base) => base.join(path).to_string_lossy().to_string(),
            None => path.to_string()}
    }
}

impl Function for Import {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let path = args
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::ArgumentCount { expected: 1, got: 0 })?;

        let key = self.cache_key(path);
        if let Some(cached) = self.import_cache.borrow().get(&key) {
            return Ok(NonFunctionValue::new(cached.clone()));
        }

        let ResolvedModule { data, .. } = self
            .resolver
            .resolve(path, self.base_path.as_deref())
            .ok_or_else(|| Error::PrimitiveError(format!("import: failed to resolve '{path}'")))?;

        self.import_cache.borrow_mut().insert(key.clone(), data.clone());
        Ok(NonFunctionValue::new(data))
    }
}

fn collect_data_functions(prefix: &str, value: &JsonValue, out: &mut HashMap<String, DataFunction>) {
    if let Some(obj) = value.as_object() {
        for (name, child) in obj {
            if let Some(fn_val) = FunctionValue::from_json(child) {
                let full = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}.{name}")
                };
                out.insert(full, DataFunction::from(&fn_val));
            }
            if child.is_object() {
                let next = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}.{name}")
                };
                collect_data_functions(&next, child, out);
            }
        }
    }
}
