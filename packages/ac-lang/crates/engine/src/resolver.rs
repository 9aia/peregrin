//! VM-backed function resolver for field interpolation and computation.
//!
//! Bridges the `FunctionResolver` trait to the VM + Host. For functions
//! with a `body:` field, evaluates the body expression by merging params
//! into the `doc` namespace. For functions without a body, dispatches to
//! the VM. Includes a recursion depth limit.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;

use ac_ast::Expr;
use ac_fields::{self, FunctionResolver};
use ac_host::DefaultHost;
use ac_resolver_core::ModuleResolver;
use ac_runtime::RuntimeValue;
use ac_schema_json::JsonSchema;
use ac_vm::{Context, Vm};

/// Bridges `FunctionResolver` to the VM + Host.
///
/// For functions with a `body:` field, evaluates the body expression
/// by merging params into the `doc` namespace and recursively resolving
/// via `eval_body`. For functions without a body, dispatches to the VM.
///
/// Includes a recursion depth limit to prevent infinite recursion
/// in user-defined functions.
pub struct VmResolver<'a> {
    pub host: DefaultHost,
    pub function_sigs: &'a [(String, ac_fields::FunctionSignature)],
    pub namespaces: ac_fields::Namespaces,
    pub depth: Cell<usize>,
    pub base_path: Option<PathBuf>,
    pub import_cache: RefCell<HashMap<String, serde_json::Value>>,
    pub imported_functions: RefCell<Vec<(String, ac_fields::FunctionSignature)>>,
    pub resolver: Option<&'a dyn ModuleResolver>,
}

const MAX_DEPTH: usize = 64;

impl<'a> FunctionResolver for VmResolver<'a> {
    fn call(&self, name: &str, args: &[serde_json::Value]) -> Result<serde_json::Value, String> {
        let depth = self.depth.get();
        if depth >= MAX_DEPTH {
            return Err(format!("maximum recursion depth ({MAX_DEPTH}) exceeded calling '{name}'"));
        }
        self.depth.set(depth + 1);
        let result = self.call_inner(name, args);
        self.depth.set(depth);
        result
    }
}

impl<'a> VmResolver<'a> {
    fn call_inner(&self, name: &str, args: &[serde_json::Value]) -> Result<serde_json::Value, String> {
        let imported_sig = self.imported_functions.borrow().iter()
            .find(|(n, _)| n == name)
            .map(|(_, s)| s.clone());
        let sig = self.function_sigs.iter()
            .find(|(n, _)| n == name)
            .map(|(_, s)| s)
            .or(imported_sig.as_ref());

        if let Some(sig) = sig {
            let param_schemas: Vec<JsonSchema> = sig
                .params
                .iter()
                .map(|(_, raw)| JsonSchema::new(raw.clone()))
                .collect();
            let args_rv: Vec<RuntimeValue> = args.iter().map(|v| RuntimeValue::new(v.clone())).collect();
            ac_fields::validate_args(&args_rv, &param_schemas)?;
        }

        if let Some(sig) = sig
            && let Some(body) = &sig.body
        {
            let params: Vec<(String, serde_json::Value)> = sig
                .params
                .iter()
                .zip(args.iter())
                .map(|((name, _), val)| (name.clone(), val.clone()))
                .collect();
            let result = ac_fields::eval_body(body, &params, &self.namespaces, self)
                .ok_or_else(|| format!("failed to evaluate body for '{name}'"))?;

            if let Some(ret_raw) = &sig.return_schema {
                let ret_schema = JsonSchema::new(ret_raw.clone());
                ac_fields::validate_return(&RuntimeValue::new(result.clone()), &ret_schema)?;
            }
            return Ok(result);
        }

        if name == "import" {
            return self.call_import(args);
        }

        let mut vm = Vm::new(self.host.clone());
        let mut ctx = Context::new();
        let expr_args: Vec<Expr<RuntimeValue>> = args
            .iter()
            .map(|v| Expr::value(RuntimeValue::new(v.clone())))
            .collect();
        let expr = Expr::call(name, expr_args);
        let result = vm.eval(&expr, &mut ctx).map_err(|e| e.to_string())?;

        if let Some(sig) = sig
            && let Some(ret_raw) = &sig.return_schema
        {
            let ret_schema = JsonSchema::new(ret_raw.clone());
            ac_fields::validate_return(&result, &ret_schema)?;
        }

        Ok(result.into_inner())
    }

    fn call_import(&self, args: &[serde_json::Value]) -> Result<serde_json::Value, String> {
        let path_str = args
            .get(0)
            .and_then(|v| v.as_str())
            .ok_or("import() requires a string path argument")?;

        let cache_key = match &self.base_path {
            Some(base) => base.join(path_str).to_string_lossy().to_string(),
            None => path_str.to_string(),
        };

        if let Some(cached) = self.import_cache.borrow().get(&cache_key) {
            return Ok(cached.clone());
        }

        let resolver = self.resolver.ok_or_else(|| {
            "import(): no module resolver configured".to_string()
        })?;

        let module = resolver
            .resolve(path_str, self.base_path.as_deref())
            .ok_or_else(|| format!("import: failed to resolve '{path_str}'"))?;

        let mut imported = self.imported_functions.borrow_mut();
        for (fn_name, fn_sig) in &module.functions {
            if !imported.iter().any(|(n, _)| n == fn_name) {
                imported.push((fn_name.clone(), fn_sig.clone()));
            }
        }

        let data = module.data.clone();
        self.import_cache
            .borrow_mut()
            .insert(cache_key, data.clone());

        Ok(data)
    }
}
