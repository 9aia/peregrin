//! Lazy AC-Lang evaluator with short-circuit `bool.if`/`and`/`or`.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use ac_ast_canonical::Expr;
use ac_dialect::Dialect;
use ac_dialect_canonical::AC_DIALECT;
use ac_fields::{APPLY_FUNCTION, DEFAULT_NAMESPACE, Namespaces, eval_expression};
use ac_runtime::{FunctionValue, NonFunctionValue};
use ac_runtime_canonical::{CanonicalContext, Context, Error, Eval, Vm};
use serde_json::Value as JsonValue;

use crate::host::TypeHost;

const MAX_DEPTH: usize = 256;
const MAX_CHAIN: usize = 128;

/// An [`Eval`] + [`FunctionResolver`] implementation that evaluates
/// AC-Lang expression strings with lazy `bool.if`, `bool.and`, `bool.or`
/// and recursive `apply` support.
///
/// Uses `ac_fields::eval_expression` for non-lazy calls, but intercepts
/// `bool.if`/`bool.and`/`bool.or` to evaluate branches lazily, preventing
/// infinite recursion in recursive function bodies.
pub struct TypeEval {
    host: TypeHost,
    namespaces: RefCell<Namespaces>,
    depth: Cell<usize>,
    call_chain: RefCell<Vec<String>>,
}

impl TypeEval {
    pub fn new(host: TypeHost) -> Self {
        TypeEval {
            host,
            namespaces: RefCell::new(HashMap::new()),
            depth: Cell::new(0),
            call_chain: RefCell::new(Vec::new()),
        }
    }

    pub fn with_namespaces(host: TypeHost, namespaces: Namespaces) -> Self {
        TypeEval {
            host,
            namespaces: RefCell::new(namespaces),
            depth: Cell::new(0),
            call_chain: RefCell::new(Vec::new()),
        }
    }

    pub(crate) fn namespaces(&self) -> std::cell::Ref<'_, Namespaces> {
        self.namespaces.borrow()
    }

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
        Ok(())
    }

    fn enter_call(&self, name: &str) {
        self.call_chain.borrow_mut().push(name.to_string());
        self.depth.set(self.depth.get() + 1);
    }

    fn exit_call(&self, prev_depth: usize) {
        self.call_chain.borrow_mut().pop();
        self.depth.set(prev_depth);
    }

    /// Lazy evaluation of an expression string.
    ///
    /// Intercepts `bool.if`, `bool.and`, `bool.or` for short-circuit
    /// evaluation. All other expressions delegate to `eval_expression`.
    pub(crate) fn eval_lazy(&self, expr: &str, namespaces: &Namespaces) -> Option<JsonValue> {
        let expr = expr.trim();
        if expr.is_empty() {
            return None;
        }

        // Handle literals (eval_expression only handles these inside args, not top-level)
        if let Some(b) = AC_DIALECT.parser().parse_bool_literal(expr) {
            return Some(JsonValue::Bool(b));
        }
        if AC_DIALECT.parser().is_null_literal(expr) {
            return Some(JsonValue::Null);
        }
        if let Some(s) = AC_DIALECT.parser().parse_string_literal(expr) {
            return Some(JsonValue::String(s));
        }
        if let Some(n) = AC_DIALECT.parser().parse_number_literal(expr) {
            return serde_json::Number::from_f64(n).map(JsonValue::Number);
        }

        // Check for lazy bool functions
        if let Some(paren_pos) = AC_DIALECT.parser().find_call_open(expr)
            && expr.ends_with(AC_DIALECT.syntax().call_close())
            && paren_pos < expr.len() - 1
        {
            let name = expr[..paren_pos].trim();
            let inner = &expr[paren_pos + 1..expr.len() - 1];
            let args = AC_DIALECT.parser().split_args(inner);

            match name {
                "bool.if" => return self.eval_bool_if(&args, namespaces),
                "bool.and" => return self.eval_bool_and(&args, namespaces),
                "bool.or" => return self.eval_bool_or(&args, namespaces),
                _ => {}
            }
        }

        // Delegate to ac_fields for everything else
        eval_expression(expr, namespaces, &self.host, &AC_DIALECT)
    }

    fn eval_bool_if(&self, args: &[String], namespaces: &Namespaces) -> Option<JsonValue> {
        if args.len() < 3 {
            return None;
        }
        let cond = self.eval_lazy(args[0].trim(), namespaces)?;
        let is_true = cond.as_bool().unwrap_or(false);
        if is_true {
            self.eval_lazy(args[1].trim(), namespaces)
        } else {
            self.eval_lazy(args[2].trim(), namespaces)
        }
    }

    fn eval_bool_and(&self, args: &[String], namespaces: &Namespaces) -> Option<JsonValue> {
        if args.len() < 2 {
            return None;
        }
        let a = self.eval_lazy(args[0].trim(), namespaces)?;
        if !a.as_bool().unwrap_or(false) {
            return Some(JsonValue::Bool(false));
        }
        self.eval_lazy(args[1].trim(), namespaces)
    }

    fn eval_bool_or(&self, args: &[String], namespaces: &Namespaces) -> Option<JsonValue> {
        if args.len() < 2 {
            return None;
        }
        let a = self.eval_lazy(args[0].trim(), namespaces)?;
        if a.as_bool().unwrap_or(false) {
            return Some(JsonValue::Bool(true));
        }
        self.eval_lazy(args[1].trim(), namespaces)
    }

    /// Handle `apply(fn_value, args_array)` — evaluate a function body
    /// with bound parameters.
    fn call_apply(&self, args: &[JsonValue]) -> Result<JsonValue, String> {
        let fn_value = args
            .first()
            .ok_or("apply() requires at least one argument (the function value)")?;
        let fn_val = FunctionValue::from_json(fn_value)
            .ok_or("apply(): first argument is not a function value")?;

        // The remaining args may be passed as individual values or as a
        // single array literal: apply(fn, [a, b]) → spread [a, b] into a, b.
        let fn_args: Vec<JsonValue> = if args.len() == 2 && args[1].is_array() {
            args[1].as_array().unwrap().clone()
        } else {
            args[1..].to_vec()
        };

        let params: Vec<(String, JsonValue)> = fn_val
            .params
            .iter()
            .zip(fn_args.iter())
            .map(|((pname, _), arg)| (pname.clone(), arg.clone()))
            .collect();

        // Build merged namespaces with params in doc namespace
        let mut merged = self.namespaces.borrow().clone();
        let mut doc_obj = merged
            .get(DEFAULT_NAMESPACE)
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        for (name, value) in &params {
            doc_obj.insert(name.clone(), value.clone());
        }
        merged.insert(DEFAULT_NAMESPACE.to_string(), JsonValue::Object(doc_obj));

        self.eval_lazy(&fn_val.body, &merged)
            .ok_or_else(|| "apply(): failed to evaluate function body".to_string())
    }

    fn call_inner(&self, name: &str, args: &[JsonValue]) -> Result<JsonValue, String> {
        if name == APPLY_FUNCTION {
            return self.call_apply(args);
        }

        // Check if name resolves to a function value in the doc namespace.
        // Handles both bare names ("is_compatible") and qualified ("doc.is_compatible").
        let (ns, key) = name.split_once('.').unwrap_or((DEFAULT_NAMESPACE, name));
        if let Some(JsonValue::Object(obj)) = self.namespaces.borrow().get(ns)
            && let Some(val) = obj.get(key)
            && FunctionValue::from_json(val).is_some()
        {
            let mut apply_args = vec![val.clone()];
            apply_args.extend(args.iter().cloned());
            return self.call_apply(&apply_args);
        }

        // Dispatch to VM via Expr tree
        let vm = Vm::new(self.host.clone());
        let mut ctx = CanonicalContext::new();
        ctx.set_eval(Box::new(TypeEvalShim {
            host: self.host.clone(),
            namespaces: self.namespaces.borrow().clone(),
            depth: Cell::new(self.depth.get()),
            call_chain: RefCell::new(self.call_chain.borrow().clone()),
        }));

        let expr_args: Vec<Expr<NonFunctionValue>> = args
            .iter()
            .map(|v| Expr::value(NonFunctionValue::new(v.clone())))
            .collect();
        let expr = Expr::call(name, expr_args);
        let result = vm.eval(&expr, &mut ctx).map_err(|e| e.to_string())?;
        Ok(result.into_inner())
    }
}

impl Eval for TypeEval {
    fn eval_expr(
        &self,
        expr: &str,
        vars: &[(String, NonFunctionValue)],
    ) -> Result<NonFunctionValue, Error> {
        // Build merged namespaces with vars in doc namespace
        let mut merged = self.namespaces.borrow().clone();
        let mut doc_obj = merged
            .get(DEFAULT_NAMESPACE)
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        for (name, value) in vars {
            doc_obj.insert(name.clone(), value.clone().into_inner());
        }
        merged.insert(DEFAULT_NAMESPACE.to_string(), JsonValue::Object(doc_obj));

        self.eval_lazy(expr, &merged)
            .map(NonFunctionValue::new)
            .ok_or_else(|| Error::PrimitiveError(format!("failed to evaluate: {expr}")))
    }
}

/// Shim for creating nested eval contexts inside VM host functions.
struct TypeEvalShim {
    host: TypeHost,
    namespaces: Namespaces,
    depth: Cell<usize>,
    call_chain: RefCell<Vec<String>>,
}

impl Eval for TypeEvalShim {
    fn eval_expr(
        &self,
        expr: &str,
        vars: &[(String, NonFunctionValue)],
    ) -> Result<NonFunctionValue, Error> {
        let eval = TypeEval {
            host: self.host.clone(),
            namespaces: RefCell::new(self.namespaces.clone()),
            depth: Cell::new(self.depth.get()),
            call_chain: RefCell::new(self.call_chain.borrow().clone()),
        };

        // Build merged namespaces with vars in doc namespace
        let mut merged = eval.namespaces.borrow().clone();
        let mut doc_obj = merged
            .get(DEFAULT_NAMESPACE)
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        for (name, value) in vars {
            doc_obj.insert(name.clone(), value.clone().into_inner());
        }
        merged.insert(DEFAULT_NAMESPACE.to_string(), JsonValue::Object(doc_obj));

        eval.eval_lazy(expr, &merged)
            .map(NonFunctionValue::new)
            .ok_or_else(|| Error::PrimitiveError(format!("failed to evaluate: {expr}")))
    }
}
