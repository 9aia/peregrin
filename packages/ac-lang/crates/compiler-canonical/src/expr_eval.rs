//! Expression evaluator for higher-order functions.
//!
//! [`ExprEval`] implements [`ac_runtime::Eval`] by bridging to the fields-layer
//! `eval_body` function. It allows VM host functions like `fp.map` to
//! evaluate function body expressions with bound parameters.
//!
//! This type is fully owned (`'static`) so it can be stored in a
//! [`ac_runtime::Context`] as a `Box<dyn Eval>`.

use std::cell::{Cell, RefCell};

use ac_ast_canonical::Expr;
use ac_dialect_canonical::AC_DIALECT;
use ac_fields::{self, APPLY_FUNCTION, FunctionResolver, Namespaces};
use ac_runtime_example_1::DefaultHost;
use ac_runtime::{FunctionValue, NonFunctionValue};
use ac_runtime_canonical::{CanonicalContext, Context, Error, Eval, Vm};

/// Fallback depth limit for non-cyclic deep recursion.
///
/// Cycle detection catches infinite recursion immediately. This limit
/// is a safety net for very deep but non-cyclic call chains (e.g.,
/// deeply nested data structures).
const MAX_DEPTH: usize = 256;

/// Maximum call chain length before assuming a cycle was missed.
const MAX_CHAIN: usize = 128;

/// An [`Eval`] implementation backed by the fields-layer expression evaluator.
///
/// Holds a snapshot of the host and namespaces, allowing it to evaluate
/// expression strings with variable bindings. Used to enable higher-order
/// functions (`fp.map`, `fp.filter`, `fp.reduce`) in the VM.
///
/// Does not support `import()` — only `apply()` and host primitives.
///
/// Termination is enforced via call-chain cycle detection: if the same
/// function name appears twice in the current call chain, recursion is
/// stopped immediately. A fallback depth limit catches non-cyclic
/// deep recursion.
pub struct ExprEval {
    host: DefaultHost,
    namespaces: Namespaces,
    depth: Cell<usize>,
    call_chain: RefCell<Vec<String>>}

impl ExprEval {
    /// Creates a new evaluator with the given host and namespaces.
    pub fn new(host: DefaultHost, namespaces: Namespaces) -> Self {
        ExprEval {
            host,
            namespaces,
            depth: Cell::new(0),
            call_chain: RefCell::new(Vec::new())}
    }

    /// Check if `name` would create a cycle in the current call chain.
    /// Returns `Err` with a descriptive message if a cycle is detected.
    fn check_cycle(&self, name: &str) -> Result<(), String> {
        let chain = self.call_chain.borrow();
        if let Some(pos) = chain.iter().position(|n| n == name) {
            let cycle: Vec<String> = chain[pos..].iter().cloned().chain(std::iter::once(name.to_string())).collect();
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

    /// Push `name` onto the call chain and increment depth.
    fn enter_call(&self, name: &str) {
        self.call_chain.borrow_mut().push(name.to_string());
        let depth = self.depth.get();
        self.depth.set(depth + 1);
    }

    /// Pop the last entry from the call chain and restore depth.
    fn exit_call(&self, prev_depth: usize) {
        self.call_chain.borrow_mut().pop();
        self.depth.set(prev_depth);
    }
}

impl FunctionResolver for ExprEval {
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
}

impl ExprEval {
    fn call_inner(
        &self,
        name: &str,
        args: &[serde_json::Value],
        type_args: &[String],
    ) -> Result<serde_json::Value, String> {
        if name == APPLY_FUNCTION {
            return self.call_apply(args);
        }

        // Host sugar: check doc.functions.<name> for a function value.
        if let Some(fn_val) = self.lookup_function_in_data(name) {
            let mut apply_args = vec![fn_val];
            apply_args.extend(args.iter().cloned());
            return self.call_apply(&apply_args);
        }

        let vm = Vm::new(self.host.clone());
        let mut ctx = CanonicalContext::new();
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

        Ok(result)
    }
}

impl Eval for ExprEval {
    fn eval_expr(
        &self,
        expr: &str,
        vars: &[(String, NonFunctionValue)],
    ) -> Result<NonFunctionValue, Error> {
        let params: Vec<(String, serde_json::Value)> = vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone().into_inner()))
            .collect();

        let result = ac_fields::eval_body(expr, &params, &self.namespaces, self, &AC_DIALECT)
            .ok_or_else(|| {
                Error::PrimitiveError(format!("failed to evaluate expression: {expr}"))
            })?;

        Ok(NonFunctionValue::new(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn cycle_detection_catches_direct_recursion() {
        let host = DefaultHost::default();
        let namespaces = HashMap::new();
        let eval = ExprEval::new(host, namespaces);

        // Simulate calling "foo" then "foo" again — should detect cycle.
        let depth = eval.depth.get();
        eval.enter_call("foo");
        assert!(eval.check_cycle("foo").is_err());
        eval.exit_call(depth);
    }

    #[test]
    fn cycle_detection_catches_mutual_recursion() {
        let host = DefaultHost::default();
        let namespaces = HashMap::new();
        let eval = ExprEval::new(host, namespaces);

        // Simulate: foo calls bar calls foo — cycle.
        let depth0 = eval.depth.get();
        eval.enter_call("foo");
        eval.enter_call("bar");
        let err = eval.check_cycle("foo").unwrap_err();
        assert!(err.contains("infinite recursion"));
        assert!(err.contains("foo"));
        assert!(err.contains("bar"));
        eval.exit_call(depth0);
    }

    #[test]
    fn cycle_detection_allows_non_cyclic_calls() {
        let host = DefaultHost::default();
        let namespaces = HashMap::new();
        let eval = ExprEval::new(host, namespaces);

        // Simulate: foo calls bar calls baz — no cycle.
        let depth0 = eval.depth.get();
        eval.enter_call("foo");
        eval.enter_call("bar");
        assert!(eval.check_cycle("baz").is_ok());
        eval.exit_call(depth0);
    }

    #[test]
    fn cycle_detection_allows_reentry_after_exit() {
        let host = DefaultHost::default();
        let namespaces = HashMap::new();
        let eval = ExprEval::new(host, namespaces);

        // foo → exit → foo again is fine (not recursive).
        let depth0 = eval.depth.get();
        eval.enter_call("foo");
        eval.exit_call(depth0);
        assert!(eval.check_cycle("foo").is_ok());
    }
}
