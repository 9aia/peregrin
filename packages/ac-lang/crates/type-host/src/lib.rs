//! # ac-type-host
//!
//! Proof-of-concept: type checking with types represented as JSON values
//! and `is_compatible` implemented as an AC-Lang function (a
//! `FunctionValue` with a body expression string) evaluated by the VM.
//!
//! This demonstrates the migration of the AC type system from Rust enums
//! to AC-Lang values: types are encoded as JSON (via `Type::to_json`),
//! and compatibility checking is performed by an AC-Lang function body
//! that uses VM primitives (`bool.if`, `str.equals`, `obj.get`, etc.)
//! rather than Rust pattern matching.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use ac_ast::Expr;
use ac_dialect::Dialect;
use ac_dialect_canonical::AC_DIALECT;
use ac_fields::{eval_expression, APPLY_FUNCTION, DEFAULT_NAMESPACE, Namespaces};
use ac_host::DefaultHost;
use ac_lib_utils::{ArrayModule, BoolModule, FnModule, ObjModule, StrModule};
use ac_runtime::{FunctionValue, NonFunctionValue};
use ac_runtime_core::ModuleHost;
use ac_vm::{Context, Error, Eval, Function, Host, Vm};
use serde_json::Value as JsonValue;

// ── Rust reference implementation (for A/B testing) ─────────────────

/// Check if `actual` is compatible with `expected`, operating on JSON-encoded types.
///
/// This mirrors `ac_typecheck_core::is_compatible` but works on
/// [`JsonValue`] instead of [`Type`].
pub fn is_compatible_json(actual: &JsonValue, expected: &JsonValue) -> bool {
    let actual_kind = kind_of(actual);
    let expected_kind = kind_of(expected);

    if actual_kind == "unknown"
        || expected_kind == "unknown"
        || actual_kind == "never"
        || expected_kind == "never"
    {
        return true;
    }

    match (actual_kind, expected_kind) {
        ("var", "var") => name_of(actual) == name_of(expected),
        ("trait", "trait") => name_of(actual) == name_of(expected),
        ("app", "app") => {
            let a_ctor = field(actual, "constructor").unwrap_or(&JsonValue::Null);
            let e_ctor = field(expected, "constructor").unwrap_or(&JsonValue::Null);
            let a_args = field(actual, "args").and_then(|v| v.as_array());
            let e_args = field(expected, "args").and_then(|v| v.as_array());
            match (a_args, e_args) {
                (Some(a), Some(e)) => {
                    is_compatible_json(a_ctor, e_ctor)
                        && a.len() == e.len()
                        && a.iter().zip(e).all(|(a, e)| is_compatible_json(a, e))
                }
                _ => false,
            }
        }
        ("string", "string") | ("number", "number") | ("boolean", "boolean") | ("null", "null") => {
            true
        }
        ("array", "array") => {
            let a_item = field(actual, "item").unwrap_or(&JsonValue::Null);
            let e_item = field(expected, "item").unwrap_or(&JsonValue::Null);
            is_compatible_json(a_item, e_item)
        }
        ("object", "object") => {
            let e_props = match field(expected, "properties").and_then(|v| v.as_object()) {
                Some(p) => p,
                None => return false,
            };
            let a_props = match field(actual, "properties").and_then(|v| v.as_object()) {
                Some(p) => p,
                None => return false,
            };
            e_props.iter().all(|(key, e_ty)| {
                a_props
                    .get(key)
                    .map(|a_ty| is_compatible_json(a_ty, e_ty))
                    .unwrap_or(false)
            })
        }
        ("function", "function") => {
            let a_params = field(actual, "params").and_then(|v| v.as_array());
            let e_params = field(expected, "params").and_then(|v| v.as_array());
            match (a_params, e_params) {
                (Some(a), Some(e)) => {
                    if a.len() != e.len() {
                        return false;
                    }
                    let a_ret = field(actual, "return").unwrap_or(&JsonValue::Null);
                    let e_ret = field(expected, "return").unwrap_or(&JsonValue::Null);
                    a.iter()
                        .zip(e)
                        .all(|(a_pair, e_pair)| {
                            let a_ty = pair_type(a_pair);
                            let e_ty = pair_type(e_pair);
                            is_compatible_json(e_ty, a_ty)
                        })
                        && is_compatible_json(a_ret, e_ret)
                }
                _ => false,
            }
        }
        _ => false,
    }
}

fn kind_of(v: &JsonValue) -> &str {
    v.as_object()
        .and_then(|o| o.get("kind"))
        .and_then(|k| k.as_str())
        .unwrap_or("")
}

fn name_of(v: &JsonValue) -> &str {
    v.as_object()
        .and_then(|o| o.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("")
}

fn field<'a>(v: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    v.as_object().and_then(|o| o.get(key))
}

fn pair_type(pair: &JsonValue) -> &JsonValue {
    pair.as_array()
        .and_then(|a| a.get(1))
        .unwrap_or(&JsonValue::Null)
}

// ── Host function: type_kind ────────────────────────────────────────

/// Host function `type_kind(type_value)` → `string`.
///
/// Extracts the `kind` field from a JSON-encoded type.
#[derive(Clone, Copy)]
pub struct TypeKind;

impl Function for TypeKind {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.is_empty() {
            return Err(Error::ArgumentCount {
                expected: 1,
                got: 0,
            });
        }
        let kind = args[0]
            .inner()
            .as_object()
            .and_then(|o| o.get("kind"))
            .and_then(|k| k.as_str())
            .unwrap_or("");
        Ok(NonFunctionValue::new(JsonValue::String(kind.to_string())))
    }
}

// ── Host function: type_name ────────────────────────────────────────

/// Host function `type_name(type_value)` → `string`.
///
/// Extracts the `name` field from a JSON-encoded type (for Var/Trait).
#[derive(Clone, Copy)]
pub struct TypeName;

impl Function for TypeName {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.is_empty() {
            return Err(Error::ArgumentCount {
                expected: 1,
                got: 0,
            });
        }
        let name = args[0]
            .inner()
            .as_object()
            .and_then(|o| o.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("");
        Ok(NonFunctionValue::new(JsonValue::String(name.to_string())))
    }
}

// ── Host function: pair_second ──────────────────────────────────────

/// Host function `pair_second(pair)` → `value`.
///
/// Extracts the second element from a 2-element array (for function params).
#[derive(Clone, Copy)]
pub struct PairSecond;

impl Function for PairSecond {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.is_empty() {
            return Err(Error::ArgumentCount {
                expected: 1,
                got: 0,
            });
        }
        let val = args[0]
            .inner()
            .as_array()
            .and_then(|a| a.get(1))
            .cloned()
            .unwrap_or(JsonValue::Null);
        Ok(NonFunctionValue::new(val))
    }
}

// ── Host function: array_zip_all ────────────────────────────────────

/// Host function `zip_all(a, b, fn)` → `bool`.
///
/// Zips two arrays and checks that fn(a_item, b_item) is true for all pairs.
/// Returns false early if arrays have different lengths.
#[derive(Clone, Copy)]
pub struct ZipAll;

impl Function for ZipAll {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let a = args[0].as_array().ok_or(Error::TypeMismatch {
            expected: "array",
            got: "non-array",
        })?;
        let b = args[1].as_array().ok_or(Error::TypeMismatch {
            expected: "array",
            got: "non-array",
        })?;
        let fn_val = args[2].as_function().ok_or(Error::TypeMismatch {
            expected: "function value",
            got: "non-function",
        })?;

        if a.len() != b.len() {
            return Ok(NonFunctionValue::new(false));
        }

        for (a_item, b_item) in a.iter().zip(b.iter()) {
            let vars: Vec<(String, NonFunctionValue)> = fn_val
                .params
                .iter()
                .zip([a_item, b_item])
                .map(|((name, _), val)| (name.clone(), NonFunctionValue::new(val.clone())))
                .collect();
            let result = ctx.eval_expr(&fn_val.body, &vars)?;
            let is_true = result.as_bool().unwrap_or(false);
            if !is_true {
                return Ok(NonFunctionValue::new(false));
            }
        }
        Ok(NonFunctionValue::new(true))
    }
}

// ── Host function: obj_every ────────────────────────────────────────

/// Host function `obj_every(actual_props, expected_props, fn)` → `bool`.
///
/// For each key in `expected_props`, checks that `actual_props` has the key
/// and `fn(actual_type, expected_type)` returns true.
#[derive(Clone, Copy)]
pub struct ObjEvery;

impl Function for ObjEvery {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let a_props = args[0].as_object().ok_or(Error::TypeMismatch {
            expected: "object",
            got: "non-object",
        })?;
        let e_props = args[1].as_object().ok_or(Error::TypeMismatch {
            expected: "object",
            got: "non-object",
        })?;
        let fn_val = args[2].as_function().ok_or(Error::TypeMismatch {
            expected: "function value",
            got: "non-function",
        })?;

        for (key, e_ty) in e_props.iter() {
            let Some(a_ty) = a_props.get(key) else {
                return Ok(NonFunctionValue::new(false));
            };
            let vars: Vec<(String, NonFunctionValue)> = fn_val
                .params
                .iter()
                .zip([a_ty, e_ty])
                .map(|((name, _), val)| (name.clone(), NonFunctionValue::new(val.clone())))
                .collect();
            let result = ctx.eval_expr(&fn_val.body, &vars)?;
            let is_true = result.as_bool().unwrap_or(false);
            if !is_true {
                return Ok(NonFunctionValue::new(false));
            }
        }
        Ok(NonFunctionValue::new(true))
    }
}

// ── Host function: func_params_compatible ───────────────────────────

/// Host function `func_params_compatible(actual_params, expected_params, fn)` → `bool`.
///
/// Checks that both param arrays have the same length, then for each pair,
/// extracts the type (second element) and calls `fn(expected_type, actual_type)`
/// — contravariant parameter compatibility.
#[derive(Clone, Copy)]
pub struct FuncParamsCompatible;

impl Function for FuncParamsCompatible {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let a_params = args[0].as_array().ok_or(Error::TypeMismatch {
            expected: "array",
            got: "non-array",
        })?;
        let e_params = args[1].as_array().ok_or(Error::TypeMismatch {
            expected: "array",
            got: "non-array",
        })?;
        let fn_val = args[2].as_function().ok_or(Error::TypeMismatch {
            expected: "function value",
            got: "non-function",
        })?;

        if a_params.len() != e_params.len() {
            return Ok(NonFunctionValue::new(false));
        }

        for (a_pair, e_pair) in a_params.iter().zip(e_params.iter()) {
            let a_ty = a_pair.as_array().and_then(|a| a.get(1)).unwrap_or(&JsonValue::Null);
            let e_ty = e_pair.as_array().and_then(|a| a.get(1)).unwrap_or(&JsonValue::Null);
            // Contravariant: is_compatible(expected_type, actual_type)
            let vars: Vec<(String, NonFunctionValue)> = fn_val
                .params
                .iter()
                .zip([e_ty, a_ty])
                .map(|((name, _), val)| (name.clone(), NonFunctionValue::new(val.clone())))
                .collect();
            let result = ctx.eval_expr(&fn_val.body, &vars)?;
            let is_true = result.as_bool().unwrap_or(false);
            if !is_true {
                return Ok(NonFunctionValue::new(false));
            }
        }
        Ok(NonFunctionValue::new(true))
    }
}

// ── Host function: app_args_compatible ──────────────────────────────

/// Host function `app_args_compatible(actual_args, expected_args, fn)` → `bool`.
///
/// Checks that both arg arrays have the same length, then for each pair,
/// calls `fn(actual_arg, expected_arg)` — covariant.
#[derive(Clone, Copy)]
pub struct AppArgsCompatible;

impl Function for AppArgsCompatible {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let a_args = args[0].as_array().ok_or(Error::TypeMismatch {
            expected: "array",
            got: "non-array",
        })?;
        let e_args = args[1].as_array().ok_or(Error::TypeMismatch {
            expected: "array",
            got: "non-array",
        })?;
        let fn_val = args[2].as_function().ok_or(Error::TypeMismatch {
            expected: "function value",
            got: "non-function",
        })?;

        if a_args.len() != e_args.len() {
            return Ok(NonFunctionValue::new(false));
        }

        for (a_item, e_item) in a_args.iter().zip(e_args.iter()) {
            let vars: Vec<(String, NonFunctionValue)> = fn_val
                .params
                .iter()
                .zip([a_item, e_item])
                .map(|((name, _), val)| (name.clone(), NonFunctionValue::new(val.clone())))
                .collect();
            let result = ctx.eval_expr(&fn_val.body, &vars)?;
            let is_true = result.as_bool().unwrap_or(false);
            if !is_true {
                return Ok(NonFunctionValue::new(false));
            }
        }
        Ok(NonFunctionValue::new(true))
    }
}

// ── TypeHost: composite host with lib-utils + type helpers ──────────

/// A host that provides type-checking helper functions alongside
/// the standard `lib-utils` modules and default `var`/`resolve`.
#[derive(Clone)]
pub struct TypeHost {
    inner: ModuleHost,
    default: DefaultHost,
}

impl TypeHost {
    pub fn new() -> Self {
        let inner = ModuleHost::new()
            .with(BoolModule)
            .with(StrModule)
            .with(ObjModule)
            .with(ArrayModule)
            .with(FnModule)
            .with(TypeHelpersModule);
        TypeHost {
            inner,
            default: DefaultHost::default(),
        }
    }
}

impl Default for TypeHost {
    fn default() -> Self {
        Self::new()
    }
}

impl Host for TypeHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        self.inner
            .function(name)
            .or_else(|| self.default.function(name))
    }

    fn lazy_function(&self, name: &str) -> Option<&dyn ac_vm::LazyFunction> {
        self.inner.lazy_function(name)
    }

    fn generic_function(&self, name: &str) -> Option<&dyn ac_vm::GenericFunction> {
        self.inner.generic_function(name)
    }
}

/// Module containing type-checking helper host functions.
#[derive(Clone, Copy)]
pub struct TypeHelpersModule;

impl ac_runtime_core::Module for TypeHelpersModule {
    fn name(&self) -> &str {
        "type"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "kind" => Some(&TypeKind),
            "name" => Some(&TypeName),
            "pair_second" => Some(&PairSecond),
            "zip_all" => Some(&ZipAll),
            "obj_every" => Some(&ObjEvery),
            "func_params_compatible" => Some(&FuncParamsCompatible),
            "app_args_compatible" => Some(&AppArgsCompatible),
            _ => None,
        }
    }
}

// ── TypeEval: Eval + FunctionResolver with lazy bool.if/and/or ──────

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

    fn check_cycle(&self, name: &str) -> Result<(), String> {
        let chain = self.call_chain.borrow();
        if let Some(pos) = chain.iter().position(|n| n == name) {
            let cycle: Vec<String> =
                chain[pos..].iter().cloned().chain(std::iter::once(name.to_string())).collect();
            return Err(format!("infinite recursion detected: {}", cycle.join(" → ")));
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
    fn eval_lazy(&self, expr: &str, namespaces: &Namespaces) -> Option<JsonValue> {
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
}



impl TypeEval {
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
        let mut ctx = Context::new();
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

// ── is_compatible as an AC-Lang FunctionValue ───────────────────────

/// The AC-Lang `is_compatible` function as a `FunctionValue`.
///
/// The body is an expression string that uses VM primitives to
/// pattern-match on JSON-encoded types. It uses `bool.if` for
/// lazy short-circuit evaluation, preventing infinite recursion.
pub fn is_compatible_function_value() -> FunctionValue {
    FunctionValue {
        params: vec![
            ("actual".to_string(), JsonValue::Null),
            ("expected".to_string(), JsonValue::Null),
        ],
        return_schema: Some(JsonValue::Bool(true)),
        body: IS_COMPATIBLE_BODY.to_string(),
    }
}

/// The expression body for `is_compatible`.
///
/// Structure:
/// 1. Wildcard check: Unknown/Never → true
/// 2. Kind-based dispatch via bool.if chain
/// 3. Recursive calls via `apply(is_compatible, [a, e])`
/// 4. Helper host functions for iteration patterns
const IS_COMPATIBLE_BODY: &str = "\
bool.if(bool.or(str.equals(type.kind(actual), \"unknown\"), str.equals(type.kind(expected), \"unknown\")), \
true, \
bool.if(bool.or(str.equals(type.kind(actual), \"never\"), str.equals(type.kind(expected), \"never\")), \
true, \
bool.if(str.equals(type.kind(actual), \"var\"), \
bool.if(str.equals(type.kind(expected), \"var\"), str.equals(type.name(actual), type.name(expected)), false), \
bool.if(str.equals(type.kind(actual), \"trait\"), \
bool.if(str.equals(type.kind(expected), \"trait\"), str.equals(type.name(actual), type.name(expected)), false), \
bool.if(str.equals(type.kind(actual), \"app\"), \
bool.if(str.equals(type.kind(expected), \"app\"), \
bool.and(apply(is_compatible, [obj.get(actual, \"constructor\"), obj.get(expected, \"constructor\")]), \
type.app_args_compatible(obj.get(actual, \"args\"), obj.get(expected, \"args\"), is_compatible)), \
false), \
bool.if(str.equals(type.kind(actual), \"string\"), \
str.equals(type.kind(expected), \"string\"), \
bool.if(str.equals(type.kind(actual), \"number\"), \
str.equals(type.kind(expected), \"number\"), \
bool.if(str.equals(type.kind(actual), \"boolean\"), \
str.equals(type.kind(expected), \"boolean\"), \
bool.if(str.equals(type.kind(actual), \"null\"), \
str.equals(type.kind(expected), \"null\"), \
bool.if(str.equals(type.kind(actual), \"array\"), \
bool.if(str.equals(type.kind(expected), \"array\"), \
apply(is_compatible, [obj.get(actual, \"item\"), obj.get(expected, \"item\")]), \
false), \
bool.if(str.equals(type.kind(actual), \"object\"), \
bool.if(str.equals(type.kind(expected), \"object\"), \
type.obj_every(obj.get(actual, \"properties\"), obj.get(expected, \"properties\"), is_compatible), \
false), \
bool.if(str.equals(type.kind(actual), \"function\"), \
bool.if(str.equals(type.kind(expected), \"function\"), \
bool.and(type.func_params_compatible(obj.get(actual, \"params\"), obj.get(expected, \"params\"), is_compatible), \
apply(is_compatible, [obj.get(actual, \"return\"), obj.get(expected, \"return\")])), \
false), \
false))))))))))))";

/// Evaluate `is_compatible` using the AC-Lang function value.
///
/// Sets up a `TypeEval` with the `is_compatible` function value in the
/// default namespace, then evaluates the body expression with `actual`
/// and `expected` bound as parameters.
pub fn eval_is_compatible_ac_lang(
    actual: &JsonValue,
    expected: &JsonValue,
) -> Result<bool, Error> {
    let host = TypeHost::new();
    let is_compatible_fn = is_compatible_function_value();

    // Build namespaces: doc.is_compatible = function value
    let mut doc_obj = serde_json::Map::new();
    doc_obj.insert(
        "is_compatible".to_string(),
        is_compatible_fn.to_json(),
    );
    let mut namespaces: Namespaces = HashMap::new();
    namespaces.insert(DEFAULT_NAMESPACE.to_string(), JsonValue::Object(doc_obj));

    let eval = TypeEval::with_namespaces(host, namespaces);

    // Build params for eval_body: actual + expected merged into doc
    let mut merged = eval.namespaces.borrow().clone();
    let mut doc = merged
        .get(DEFAULT_NAMESPACE)
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    doc.insert("actual".to_string(), actual.clone());
    doc.insert("expected".to_string(), expected.clone());
    merged.insert(DEFAULT_NAMESPACE.to_string(), JsonValue::Object(doc));

    let result = eval
        .eval_lazy(IS_COMPATIBLE_BODY, &merged)
        .ok_or_else(|| Error::PrimitiveError("failed to evaluate is_compatible body".to_string()))?;

    result.as_bool().ok_or(Error::TypeMismatch {
        expected: "boolean",
        got: "non-boolean",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_typecheck_core::{Type, is_compatible as is_compatible_rust};

    fn check(a: Type, e: Type) {
        let a_json = a.to_json();
        let e_json = e.to_json();
        let rust_result = is_compatible_rust(&a, &e);
        let json_result = is_compatible_json(&a_json, &e_json);
        assert_eq!(
            rust_result, json_result,
            "mismatch: is_compatible({:?}, {:?}) — rust={}, json={}",
            a, e, rust_result, json_result
        );
    }

    #[test]
    fn unknown_compatible_with_everything() {
        check(Type::Unknown, Type::String);
        check(Type::Number, Type::Unknown);
        check(Type::Unknown, Type::Unknown);
    }

    #[test]
    fn never_compatible_with_everything() {
        check(Type::Never, Type::String);
        check(Type::Number, Type::Never);
    }

    #[test]
    fn exact_matches() {
        check(Type::String, Type::String);
        check(Type::Number, Type::Number);
        check(Type::String, Type::Number);
    }

    #[test]
    fn array_compatibility() {
        check(
            Type::Array(Box::new(Type::String)),
            Type::Array(Box::new(Type::String)),
        );
        check(
            Type::Array(Box::new(Type::String)),
            Type::Array(Box::new(Type::Number)),
        );
    }

    #[test]
    fn object_allows_extra_properties() {
        let mut a = std::collections::BTreeMap::new();
        a.insert("name".to_string(), Type::String);
        a.insert("extra".to_string(), Type::Number);

        let mut e = std::collections::BTreeMap::new();
        e.insert("name".to_string(), Type::String);

        check(Type::Object(a), Type::Object(e));
    }

    #[test]
    fn object_missing_property() {
        let a = std::collections::BTreeMap::new();
        let mut e = std::collections::BTreeMap::new();
        e.insert("name".to_string(), Type::String);

        check(Type::Object(a), Type::Object(e));
    }

    #[test]
    fn var_compatibility() {
        check(Type::Var("T".to_string()), Type::Var("T".to_string()));
        check(Type::Var("T".to_string()), Type::Var("U".to_string()));
        check(Type::Var("T".to_string()), Type::String);
    }

    #[test]
    fn trait_compatibility() {
        check(
            Type::Trait("Numeric".to_string()),
            Type::Trait("Numeric".to_string()),
        );
        check(
            Type::Trait("Numeric".to_string()),
            Type::Trait("Comparable".to_string()),
        );
    }

    #[test]
    fn function_compatibility() {
        let a_fn = ac_typecheck_core::FunctionType {
            type_params: vec![],
            params: vec![("x".to_string(), Type::String)],
            return_type: Box::new(Type::Number),
        };
        let e_fn = ac_typecheck_core::FunctionType {
            type_params: vec![],
            params: vec![("y".to_string(), Type::String)],
            return_type: Box::new(Type::Number),
        };
        check(Type::Function(a_fn), Type::Function(e_fn));

        let e_fn2 = ac_typecheck_core::FunctionType {
            type_params: vec![],
            params: vec![("y".to_string(), Type::Number)],
            return_type: Box::new(Type::Number),
        };
        check(
            Type::Function(ac_typecheck_core::FunctionType {
                type_params: vec![],
                params: vec![("x".to_string(), Type::String)],
                return_type: Box::new(Type::Number),
            }),
            Type::Function(e_fn2),
        );
    }

    #[test]
    fn app_compatibility() {
        check(
            Type::App(Box::new(Type::Var("F".to_string())), vec![Type::String]),
            Type::App(Box::new(Type::Var("F".to_string())), vec![Type::String]),
        );
        check(
            Type::App(Box::new(Type::Var("F".to_string())), vec![Type::String]),
            Type::App(Box::new(Type::Var("F".to_string())), vec![Type::Number]),
        );
    }

    #[test]
    fn to_json_roundtrip() {
        let types = vec![
            Type::String,
            Type::Number,
            Type::Boolean,
            Type::Null,
            Type::Unknown,
            Type::Never,
            Type::Var("T".to_string()),
            Type::Trait("Numeric".to_string()),
            Type::Array(Box::new(Type::String)),
            Type::App(
                Box::new(Type::Var("F".to_string())),
                vec![Type::String, Type::Number],
            ),
        ];
        for ty in &types {
            let json = ty.to_json();
            let recovered = Type::from_json(&json).unwrap();
            assert_eq!(ty, &recovered, "roundtrip failed for {:?}", ty);
        }
    }

    // ── AC-Lang A/B tests ───────────────────────────────────────────
    // Compare eval_is_compatible_ac_lang against the Rust reference.

    fn check_ac_lang(a: Type, e: Type) {
        let a_json = a.to_json();
        let e_json = e.to_json();
        let rust_result = is_compatible_rust(&a, &e);
        let ac_lang_result = eval_is_compatible_ac_lang(&a_json, &e_json)
            .unwrap_or_else(|err| panic!("AC-Lang eval failed for ({a:?}, {e:?}): {err:?}"));
        assert_eq!(
            rust_result, ac_lang_result,
            "A/B mismatch: is_compatible({a:?}, {e:?}) — rust={rust_result}, ac_lang={ac_lang_result}"
        );
    }

    #[test]
    fn ac_lang_unknown_compatible_with_everything() {
        check_ac_lang(Type::Unknown, Type::String);
        check_ac_lang(Type::Number, Type::Unknown);
        check_ac_lang(Type::Unknown, Type::Unknown);
    }

    #[test]
    fn ac_lang_never_compatible_with_everything() {
        check_ac_lang(Type::Never, Type::String);
        check_ac_lang(Type::Number, Type::Never);
    }

    #[test]
    fn ac_lang_exact_matches() {
        check_ac_lang(Type::String, Type::String);
        check_ac_lang(Type::Number, Type::Number);
        check_ac_lang(Type::String, Type::Number);
    }

    #[test]
    fn ac_lang_array_compatibility() {
        check_ac_lang(
            Type::Array(Box::new(Type::String)),
            Type::Array(Box::new(Type::String)),
        );
        check_ac_lang(
            Type::Array(Box::new(Type::String)),
            Type::Array(Box::new(Type::Number)),
        );
    }

    #[test]
    fn ac_lang_object_allows_extra_properties() {
        let mut a = std::collections::BTreeMap::new();
        a.insert("name".to_string(), Type::String);
        a.insert("extra".to_string(), Type::Number);

        let mut e = std::collections::BTreeMap::new();
        e.insert("name".to_string(), Type::String);

        check_ac_lang(Type::Object(a), Type::Object(e));
    }

    #[test]
    fn ac_lang_object_missing_property() {
        let a = std::collections::BTreeMap::new();
        let mut e = std::collections::BTreeMap::new();
        e.insert("name".to_string(), Type::String);

        check_ac_lang(Type::Object(a), Type::Object(e));
    }

    #[test]
    fn ac_lang_var_compatibility() {
        check_ac_lang(Type::Var("T".to_string()), Type::Var("T".to_string()));
        check_ac_lang(Type::Var("T".to_string()), Type::Var("U".to_string()));
        check_ac_lang(Type::Var("T".to_string()), Type::String);
    }

    #[test]
    fn ac_lang_trait_compatibility() {
        check_ac_lang(
            Type::Trait("Numeric".to_string()),
            Type::Trait("Numeric".to_string()),
        );
        check_ac_lang(
            Type::Trait("Numeric".to_string()),
            Type::Trait("Comparable".to_string()),
        );
    }

    #[test]
    fn ac_lang_function_compatibility() {
        let a_fn = ac_typecheck_core::FunctionType {
            type_params: vec![],
            params: vec![("x".to_string(), Type::String)],
            return_type: Box::new(Type::Number),
        };
        let e_fn = ac_typecheck_core::FunctionType {
            type_params: vec![],
            params: vec![("y".to_string(), Type::String)],
            return_type: Box::new(Type::Number),
        };
        check_ac_lang(Type::Function(a_fn), Type::Function(e_fn));

        let e_fn2 = ac_typecheck_core::FunctionType {
            type_params: vec![],
            params: vec![("y".to_string(), Type::Number)],
            return_type: Box::new(Type::Number),
        };
        check_ac_lang(
            Type::Function(ac_typecheck_core::FunctionType {
                type_params: vec![],
                params: vec![("x".to_string(), Type::String)],
                return_type: Box::new(Type::Number),
            }),
            Type::Function(e_fn2),
        );
    }

    #[test]
    fn ac_lang_app_compatibility() {
        check_ac_lang(
            Type::App(Box::new(Type::Var("F".to_string())), vec![Type::String]),
            Type::App(Box::new(Type::Var("F".to_string())), vec![Type::String]),
        );
        check_ac_lang(
            Type::App(Box::new(Type::Var("F".to_string())), vec![Type::String]),
            Type::App(Box::new(Type::Var("F".to_string())), vec![Type::Number]),
        );
    }
}
