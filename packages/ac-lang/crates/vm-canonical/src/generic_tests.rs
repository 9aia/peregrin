use crate::RuntimeContext;
use crate::Context;
use crate::Error;
use crate::{Function, GenericFunction, Host};
use crate::Vm;
use crate::Expr;
use crate::NonFunctionValue;

// ── test host with a generic identity function ─────────────────────

#[derive(Clone)]
struct TypedIdentity;

impl Function for TypedIdentity {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn RuntimeContext,
    ) -> Result<NonFunctionValue, Error> {
        Ok(args.first().cloned().unwrap_or_default())
    }
}

impl GenericFunction for TypedIdentity {
    fn call_with_types(
        &self,
        args: &[NonFunctionValue],
        type_args: &[String],
        _ctx: &mut dyn RuntimeContext,
    ) -> Result<NonFunctionValue, Error> {
        // Return a string describing the type args to prove dispatch worked.
        let type_str = type_args.join(", ");
        Ok(NonFunctionValue::new(format!(
            "identity<{}>({})",
            type_str,
            args.first()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .or_else(|| args.first().and_then(|v| v.as_number().map(|n| n.to_string())))
                .unwrap_or_else(|| "?".to_string())
        )))
    }
}

#[derive(Clone)]
struct GenericHost {
    identity: TypedIdentity,
}

impl Host for GenericHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "identity" => Some(&self.identity),
            _ => None,
        }
    }

    fn generic_function(&self, name: &str) -> Option<&dyn GenericFunction> {
        match name {
            "identity" => Some(&self.identity),
            _ => None,
        }
    }
}

impl Default for GenericHost {
    fn default() -> Self {
        GenericHost {
            identity: TypedIdentity,
        }
    }
}

// ── tests ──────────────────────────────────────────────────────────

#[test]
fn generic_function_dispatch_with_type_args() {
    let vm = Vm::new(GenericHost::default());
    let mut ctx = Context::new();

    let expr = Expr::call_with_type_args(
        "identity",
        vec![Expr::value(NonFunctionValue::new("hello"))],
        vec!["string".to_string()],
    );

    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_str(), Some("identity<string>(hello)"));
}

#[test]
fn generic_function_dispatch_with_multiple_type_args() {
    let vm = Vm::new(GenericHost::default());
    let mut ctx = Context::new();

    let expr = Expr::call_with_type_args(
        "identity",
        vec![Expr::value(NonFunctionValue::new(42.0))],
        vec!["number".to_string(), "boolean".to_string()],
    );

    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_str(), Some("identity<number, boolean>(42)"));
}

#[test]
fn generic_function_falls_back_to_function_when_no_type_args() {
    let vm = Vm::new(GenericHost::default());
    let mut ctx = Context::new();

    // No type_args → should use regular Function::call, not GenericFunction.
    let expr = Expr::call("identity", vec![Expr::value(NonFunctionValue::new("hello"))]);

    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_str(), Some("hello"));
}

#[test]
fn generic_function_falls_back_to_function_when_not_registered() {
    // Host has generic_function but returns None for unknown name.
    let vm = Vm::new(GenericHost::default());
    let mut ctx = Context::new();

    // "unknown" is not registered as a function or generic function.
    let expr = Expr::call_with_type_args(
        "unknown",
        vec![Expr::value(NonFunctionValue::new(1.0))],
        vec!["number".to_string()],
    );

    let result = vm.eval(&expr, &mut ctx);
    assert!(result.is_err());
}

// ── test with a generic function that uses type_args for behavior ──

#[derive(Clone)]
struct TypedArrayify;

impl Function for TypedArrayify {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn RuntimeContext,
    ) -> Result<NonFunctionValue, Error> {
        // Default: wrap in array
        Ok(NonFunctionValue::new(serde_json::json!([args
            .first()
            .map(|v| v.inner().clone())
            .unwrap_or(serde_json::Value::Null)])))
    }
}

impl GenericFunction for TypedArrayify {
    fn call_with_types(
        &self,
        args: &[NonFunctionValue],
        type_args: &[String],
        _ctx: &mut dyn RuntimeContext,
    ) -> Result<NonFunctionValue, Error> {
        let val = args.first().map(|v| v.inner().clone()).unwrap_or(serde_json::Value::Null);

        // If type_args indicates "string", ensure the result is stringified.
        if type_args.first().is_some_and(|t| t == "string") {
            let s = match &val {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            return Ok(NonFunctionValue::new(s));
        }

        // Default: wrap in array
        Ok(NonFunctionValue::new(serde_json::json!([val])))
    }
}

#[derive(Clone)]
struct ArrayifyHost;

impl Host for ArrayifyHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "arrayify" => Some(&TypedArrayify),
            _ => None,
        }
    }

    fn generic_function(&self, name: &str) -> Option<&dyn GenericFunction> {
        match name {
            "arrayify" => Some(&TypedArrayify),
            _ => None,
        }
    }
}

#[test]
fn generic_function_type_args_affect_behavior() {
    let vm = Vm::new(ArrayifyHost);
    let mut ctx = Context::new();

    // With type_args=["string"], should stringify the input.
    let expr = Expr::call_with_type_args(
        "arrayify",
        vec![Expr::value(NonFunctionValue::new(42.0))],
        vec!["string".to_string()],
    );

    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_str(), Some("42.0"));
}

#[test]
fn generic_function_without_type_args_uses_default_behavior() {
    let vm = Vm::new(ArrayifyHost);
    let mut ctx = Context::new();

    // Without type_args, should wrap in array (default Function behavior).
    let expr = Expr::call("arrayify", vec![Expr::value(NonFunctionValue::new(42.0))]);

    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert!(result.as_array().is_some());
    assert_eq!(
        result.as_array().unwrap().first(),
        Some(&serde_json::json!(42.0))
    );
}
