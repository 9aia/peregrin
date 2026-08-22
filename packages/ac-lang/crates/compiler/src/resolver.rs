//! VM-backed function resolver for field interpolation and computation.
//!
//! Bridges the `FunctionResolver` trait to the VM + Host. For `apply()`,
//! extracts a function value and evaluates its body. For `import()`,
//! resolves and loads external documents. For other functions, dispatches
//! to the VM. Includes a recursion depth limit.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;

use ac_ast::Expr;
use ac_dialect::Dialect;
use ac_dialect_canonical::AC_DIALECT;
use ac_fields::{self, APPLY_FUNCTION, FunctionResolver};
use ac_host::DefaultHost;
use ac_module_resolver_core::{IMPORT_FUNCTION, ModuleResolver};
use ac_runtime::{FunctionValue, NonFunctionValue};
use ac_typecheck_core::TypeEnv;
use ac_typecheck_infer::{FunctionTable, infer_with_type_args};
use ac_vm::{Context, Vm};

use crate::expr_eval::ExprEval;

/// Bridges `FunctionResolver` to the VM + Host.
///
/// For `apply()`, extracts a function value (object with `body` field)
/// and evaluates its body with bound arguments. For `import()`, loads
/// external documents. For other names, checks `doc.functions.<name>`
/// for a function value (host sugar convention) and dispatches via
/// `apply` if found. Otherwise dispatches to the VM.
///
/// Termination is enforced via call-chain cycle detection: if the same
/// function name appears twice in the current call chain, recursion is
/// stopped immediately with a descriptive error showing the cycle.
/// A fallback depth limit catches non-cyclic deep recursion.
pub struct VmResolver<'a> {
    pub host: DefaultHost,
    pub namespaces: ac_fields::Namespaces,
    pub depth: Cell<usize>,
    pub call_chain: RefCell<Vec<String>>,
    pub base_path: Option<PathBuf>,
    pub import_cache: RefCell<HashMap<String, serde_json::Value>>,
    pub resolver: Option<&'a dyn ModuleResolver>,
    /// Optional typechecker context for resolving generic type arguments.
    /// When set, `type_args_for` uses this to infer concrete type arguments
    /// for generic function calls.
    pub type_env: Option<TypeEnv>,
    /// Optional function table for resolving generic type arguments.
    pub function_table: Option<FunctionTable>,
}

/// Fallback depth limit for non-cyclic deep recursion.
const MAX_DEPTH: usize = 256;

/// Maximum call chain length before assuming a cycle was missed.
const MAX_CHAIN: usize = 128;

impl<'a> VmResolver<'a> {
    /// Check if `name` would create a cycle in the current call chain.
    fn check_cycle(&self, name: &str) -> Result<(), String> {
        let chain = self.call_chain.borrow();
        if let Some(pos) = chain.iter().position(|n| n == name) {
            let cycle: Vec<String> = chain[pos..]
                .iter()
                .cloned()
                .chain(std::iter::once(name.to_string()))
                .collect();
            return Err(format!(
                "infinite recursion detected: {}",
                cycle.join(" → ")
            ));
        }
        if chain.len() >= MAX_CHAIN {
            return Err(format!(
                "call chain too long ({MAX_CHAIN}) — possible infinite recursion"
            ));
        }
        drop(chain);
        Ok(())
    }

    fn enter_call(&self, name: &str) {
        self.call_chain.borrow_mut().push(name.to_string());
        let depth = self.depth.get();
        self.depth.set(depth + 1);
    }

    fn exit_call(&self, prev_depth: usize) {
        self.call_chain.borrow_mut().pop();
        self.depth.set(prev_depth);
    }
}

impl<'a> FunctionResolver for VmResolver<'a> {
    fn call(&self, name: &str, args: &[serde_json::Value]) -> Result<serde_json::Value, String> {
        let depth = self.depth.get();
        if depth >= MAX_DEPTH {
            return Err(format!(
                "maximum recursion depth ({MAX_DEPTH}) exceeded calling '{name}'"
            ));
        }
        self.check_cycle(name)?;
        self.enter_call(name);
        let result = self.call_inner(name, args, &[]);
        self.exit_call(depth);
        result
    }

    fn call_with_type_args(
        &self,
        name: &str,
        args: &[serde_json::Value],
        type_args: &[String],
    ) -> Result<serde_json::Value, String> {
        let depth = self.depth.get();
        if depth >= MAX_DEPTH {
            return Err(format!(
                "maximum recursion depth ({MAX_DEPTH}) exceeded calling '{name}'"
            ));
        }
        self.check_cycle(name)?;
        self.enter_call(name);
        let result = self.call_inner(name, args, type_args);
        self.exit_call(depth);
        result
    }

    fn type_args_for(&self, expr: &str) -> Vec<String> {
        let (Some(env), Some(table)) = (&self.type_env, &self.function_table) else {
            return Vec::new();
        };
        infer_with_type_args(expr, env, table, &AC_DIALECT).type_args
    }
}

impl<'a> VmResolver<'a> {
    fn call_inner(
        &self,
        name: &str,
        args: &[serde_json::Value],
        type_args: &[String],
    ) -> Result<serde_json::Value, String> {
        if name == IMPORT_FUNCTION {
            return self.call_import(args);
        }

        if name == APPLY_FUNCTION {
            return self.call_apply(args);
        }

        // Host sugar: check doc.functions.<name> for a function value.
        // The `functions:` section is a runtime convention — it stays in
        // data as regular fields, and the host looks here for callable
        // function values before dispatching to the VM.
        if let Some(fn_val) = self.lookup_function_in_data(name) {
            let mut apply_args = vec![fn_val];
            apply_args.extend(args.iter().cloned());
            return self.call_apply(&apply_args);
        }

        let vm = Vm::new(self.host.clone());
        let mut ctx = Context::new();
        let eval = ExprEval::new(self.host.clone(), self.namespaces.clone());
        ctx.set_eval(Box::new(eval));
        let expr_args: Vec<Expr<NonFunctionValue>> = args
            .iter()
            .map(|v| Expr::value(NonFunctionValue::new(v.clone())))
            .collect();
        let expr = if type_args.is_empty() {
            Expr::call(name, expr_args)
        } else {
            Expr::call_with_type_args(name, expr_args, type_args.to_vec())
        };
        let result = vm.eval(&expr, &mut ctx).map_err(|e| e.to_string())?;

        Ok(result.into_inner())
    }

    /// Looks up `name` in `doc.functions.<name>` within the namespaces.
    /// Returns the function value if it's a valid function (has a `body` field).
    fn lookup_function_in_data(&self, name: &str) -> Option<serde_json::Value> {
        let doc = self.namespaces.get(ac_fields::DEFAULT_NAMESPACE)?;
        let functions = doc.get("functions")?;
        let fn_val = functions.get(name)?;
        if FunctionValue::from_json(fn_val).is_some() {
            Some(fn_val.clone())
        } else {
            None
        }
    }

    fn call_apply(&self, args: &[serde_json::Value]) -> Result<serde_json::Value, String> {
        let fn_value = args
            .first()
            .ok_or("apply() requires at least one argument (the function value)")?;
        let fn_val = FunctionValue::from_json(fn_value)
            .ok_or("apply(): first argument is not a function value")?;

        let fn_args = &args[1..];
        let params: Vec<(String, serde_json::Value)> = fn_val
            .params
            .iter()
            .zip(fn_args.iter())
            .map(|((pname, _), arg)| (pname.clone(), arg.clone()))
            .collect();

        let result =
            ac_fields::eval_body(&fn_val.body, &params, &self.namespaces, self, &AC_DIALECT)
                .ok_or_else(|| "apply(): failed to evaluate function body".to_string())?;

        if let Some(ret_raw) = &fn_val.return_schema {
            let ret_schema = AC_DIALECT
                .schema_vocabulary()
                .compile(ret_raw)
                .map_err(|e| format!("invalid return schema: {e}"))?;
            ac_fields::validate_return(&NonFunctionValue::new(result.clone()), ret_schema.as_ref())?;
        }

        Ok(result)
    }

    fn call_import(&self, args: &[serde_json::Value]) -> Result<serde_json::Value, String> {
        let path_str = args
            .first()
            .and_then(|v| v.as_str())
            .ok_or("import() requires a string path argument")?;

        let cache_key = match &self.base_path {
            Some(base) => base.join(path_str).to_string_lossy().to_string(),
            None => path_str.to_string(),
        };

        if let Some(cached) = self.import_cache.borrow().get(&cache_key) {
            return Ok(cached.clone());
        }

        let resolver = self
            .resolver
            .ok_or_else(|| "import(): no module resolver configured".to_string())?;

        let module = resolver
            .resolve(path_str, self.base_path.as_deref())
            .ok_or_else(|| format!("import: failed to resolve '{path_str}'"))?;

        let data = module.data.clone();
        self.import_cache
            .borrow_mut()
            .insert(cache_key, data.clone());

        Ok(data)
    }
}
