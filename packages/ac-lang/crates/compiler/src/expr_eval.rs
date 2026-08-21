//! Expression evaluator for higher-order functions.
//!
//! [`ExprEval`] implements [`ac_vm::Eval`] by bridging to the fields-layer
//! `eval_body` function. It allows VM host functions like `fp.map` to
//! evaluate function body expressions with bound parameters.
//!
//! This type is fully owned (`'static`) so it can be stored in a
//! [`ac_vm::Context`] as a `Box<dyn Eval>`.

use std::cell::Cell;

use ac_ast::Expr;
use ac_dialect_canonical::AC_DIALECT;
use ac_fields::{self, APPLY_FUNCTION, FunctionResolver, Namespaces};
use ac_host::DefaultHost;
use ac_runtime::{FunctionValue, RuntimeValue};
use ac_vm::{Context, Error, Eval, Vm};

const MAX_DEPTH: usize = 64;

/// An [`Eval`] implementation backed by the fields-layer expression evaluator.
///
/// Holds a snapshot of the host and namespaces, allowing it to evaluate
/// expression strings with variable bindings. Used to enable higher-order
/// functions (`fp.map`, `fp.filter`, `fp.reduce`) in the VM.
///
/// Does not support `import()` — only `apply()` and host primitives.
pub struct ExprEval {
    host: DefaultHost,
    namespaces: Namespaces,
    depth: Cell<usize>,
}

impl ExprEval {
    /// Creates a new evaluator with the given host and namespaces.
    pub fn new(host: DefaultHost, namespaces: Namespaces) -> Self {
        ExprEval {
            host,
            namespaces,
            depth: Cell::new(0),
        }
    }
}

impl FunctionResolver for ExprEval {
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

impl ExprEval {
    fn call_inner(&self, name: &str, args: &[serde_json::Value]) -> Result<serde_json::Value, String> {
        if name == APPLY_FUNCTION {
            return self.call_apply(args);
        }

        // Host sugar: check doc.functions.<name> for a function value.
        if let Some(fn_val) = self.lookup_function_in_data(name) {
            let mut apply_args = vec![fn_val];
            apply_args.extend(args.iter().cloned());
            return self.call_apply(&apply_args);
        }

        let mut vm = Vm::new(self.host.clone());
        let mut ctx = Context::new();
        let expr_args: Vec<Expr<RuntimeValue>> = args
            .iter()
            .map(|v| Expr::value(RuntimeValue::new(v.clone())))
            .collect();
        let expr = Expr::call(name, expr_args);
        let result = vm.eval(&expr, &mut ctx).map_err(|e| e.to_string())?;
        Ok(result.into_inner())
    }

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

        let result = ac_fields::eval_body(&fn_val.body, &params, &self.namespaces, self, &AC_DIALECT)
            .ok_or_else(|| "apply(): failed to evaluate function body".to_string())?;

        Ok(result)
    }
}

impl Eval for ExprEval {
    fn eval_expr(
        &self,
        expr: &str,
        vars: &[(String, RuntimeValue)],
    ) -> Result<RuntimeValue, Error> {
        let params: Vec<(String, serde_json::Value)> = vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone().into_inner()))
            .collect();

        let result = ac_fields::eval_body(expr, &params, &self.namespaces, self, &AC_DIALECT)
            .ok_or_else(|| Error::PrimitiveError(format!("failed to evaluate expression: {expr}")))?;

        Ok(RuntimeValue::new(result))
    }
}
