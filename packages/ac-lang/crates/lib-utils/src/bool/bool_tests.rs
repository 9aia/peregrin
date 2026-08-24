use ac_runtime_values::NonFunctionValue;

use ac_runtime_values::{Error};

use ac_runtime_modules::Module;

use super::bool::BoolModule;

fn call(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(
    module: &dyn Module,
    name: &str,
    args: &[NonFunctionValue],
) -> Result<NonFunctionValue, Error> {
    let func = module.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx)
}

fn b(v: bool) -> NonFunctionValue {
    NonFunctionValue::new(v)
}

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

#[test]
fn module_name() {
    assert_eq!(BoolModule.name(), "bool");
}

#[test]
fn and_true() {
    assert_eq!(
        call(&BoolModule, "and", &[b(true), b(true)]).as_bool(),
        Some(true)
    );
}

#[test]
fn and_false() {
    assert_eq!(
        call(&BoolModule, "and", &[b(true), b(false)]).as_bool(),
        Some(false)
    );
    assert_eq!(
        call(&BoolModule, "and", &[b(false), b(true)]).as_bool(),
        Some(false)
    );
    assert_eq!(
        call(&BoolModule, "and", &[b(false), b(false)]).as_bool(),
        Some(false)
    );
}

#[test]
fn or_true() {
    assert_eq!(
        call(&BoolModule, "or", &[b(true), b(false)]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&BoolModule, "or", &[b(false), b(true)]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&BoolModule, "or", &[b(true), b(true)]).as_bool(),
        Some(true)
    );
}

#[test]
fn or_false() {
    assert_eq!(
        call(&BoolModule, "or", &[b(false), b(false)]).as_bool(),
        Some(false)
    );
}

#[test]
fn not_true() {
    assert_eq!(call(&BoolModule, "not", &[b(true)]).as_bool(), Some(false));
}

#[test]
fn not_false() {
    assert_eq!(call(&BoolModule, "not", &[b(false)]).as_bool(), Some(true));
}

#[test]
fn xor_true() {
    assert_eq!(
        call(&BoolModule, "xor", &[b(true), b(false)]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&BoolModule, "xor", &[b(false), b(true)]).as_bool(),
        Some(true)
    );
}

#[test]
fn xor_false() {
    assert_eq!(
        call(&BoolModule, "xor", &[b(true), b(true)]).as_bool(),
        Some(false)
    );
    assert_eq!(
        call(&BoolModule, "xor", &[b(false), b(false)]).as_bool(),
        Some(false)
    );
}

#[test]
fn nand_true() {
    assert_eq!(
        call(&BoolModule, "nand", &[b(true), b(false)]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&BoolModule, "nand", &[b(false), b(false)]).as_bool(),
        Some(true)
    );
}

#[test]
fn nand_false() {
    assert_eq!(
        call(&BoolModule, "nand", &[b(true), b(true)]).as_bool(),
        Some(false)
    );
}

#[test]
fn nor_true() {
    assert_eq!(
        call(&BoolModule, "nor", &[b(false), b(false)]).as_bool(),
        Some(true)
    );
}

#[test]
fn nor_false() {
    assert_eq!(
        call(&BoolModule, "nor", &[b(true), b(false)]).as_bool(),
        Some(false)
    );
    assert_eq!(
        call(&BoolModule, "nor", &[b(true), b(true)]).as_bool(),
        Some(false)
    );
}

#[test]
fn implies_true() {
    assert_eq!(
        call(&BoolModule, "implies", &[b(false), b(false)]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&BoolModule, "implies", &[b(false), b(true)]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&BoolModule, "implies", &[b(true), b(true)]).as_bool(),
        Some(true)
    );
}

#[test]
fn implies_false() {
    assert_eq!(
        call(&BoolModule, "implies", &[b(true), b(false)]).as_bool(),
        Some(false)
    );
}

#[test]
fn eq_true() {
    assert_eq!(
        call(&BoolModule, "eq", &[b(true), b(true)]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&BoolModule, "eq", &[b(false), b(false)]).as_bool(),
        Some(true)
    );
}

#[test]
fn eq_false() {
    assert_eq!(
        call(&BoolModule, "eq", &[b(true), b(false)]).as_bool(),
        Some(false)
    );
    assert_eq!(
        call(&BoolModule, "eq", &[b(false), b(true)]).as_bool(),
        Some(false)
    );
}

#[test]
fn unknown_function_returns_none() {
    assert!(BoolModule.function("nope").is_none());
}

#[test]
fn type_mismatch_on_non_bool() {
    let func = BoolModule.function("not").unwrap();
    let mut ctx = CanonicalContext::new();
    let result = func.call(&[n(42.0)], &mut ctx);
    assert_eq!(
        result,
        Err(Error::TypeMismatch {
            expected: "boolean",
            got: "non-boolean"
        })
    );
}

#[test]
fn argument_count_on_missing_args() {
    let func = BoolModule.function("not").unwrap();
    let mut ctx = CanonicalContext::new();
    let result = func.call(&[], &mut ctx);
    assert_eq!(
        result,
        Err(Error::ArgumentCount {
            expected: 1,
            got: 0
        })
    );
}

// ── lazy evaluation tests ─────────────────────────────────────────

use ac_ast_canonical::Expr;
use ac_runtime_canonical::{CanonicalContext, Host, Vm};
use ac_runtime_modules::ModuleHost;

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(BoolModule);
    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

fn eval_expr_result(expr: &Expr<NonFunctionValue>) -> Result<NonFunctionValue, Error> {
    let host = ModuleHost::new().with(BoolModule);
    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();
    vm.eval(expr, &mut ctx)
}

#[test]
fn lazy_if_true_evaluates_only_then_branch() {
    let expr = Expr::call(
        "bool.if",
        vec![
            Expr::value(b(true)),
            Expr::value(n(1.0)),
            Expr::call("nonexistent", vec![]),
        ],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_number(), Some(1.0));
}

#[test]
fn lazy_if_false_evaluates_only_else_branch() {
    let expr = Expr::call(
        "bool.if",
        vec![
            Expr::value(b(false)),
            Expr::call("nonexistent", vec![]),
            Expr::value(n(2.0)),
        ],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_number(), Some(2.0));
}

#[test]
fn lazy_and_short_circuits_on_false() {
    let expr = Expr::call(
        "bool.and",
        vec![Expr::value(b(false)), Expr::call("nonexistent", vec![])],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn lazy_or_short_circuits_on_true() {
    let expr = Expr::call(
        "bool.or",
        vec![Expr::value(b(true)), Expr::call("nonexistent", vec![])],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn lazy_and_evaluates_both_when_first_is_true() {
    let expr = Expr::call("bool.and", vec![Expr::value(b(true)), Expr::value(b(true))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn lazy_or_evaluates_both_when_first_is_false() {
    let expr = Expr::call("bool.or", vec![Expr::value(b(false)), Expr::value(b(true))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn lazy_function_resolves_via_module_host() {
    let host = ModuleHost::new().with(BoolModule);
    assert!(host.lazy_function("bool.if").is_some());
    assert!(host.lazy_function("bool.and").is_some());
    assert!(host.lazy_function("bool.or").is_some());
    assert!(host.lazy_function("bool.not").is_none());
}

// ── additional edge-case tests ─────────────────────────────────────

// Argument count errors for binary functions

#[test]
fn and_no_args_errors() {
    let result = call_err(&BoolModule, "and", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn and_one_arg_errors() {
    let result = call_err(&BoolModule, "and", &[b(true)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn or_no_args_errors() {
    let result = call_err(&BoolModule, "or", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn or_one_arg_errors() {
    let result = call_err(&BoolModule, "or", &[b(true)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn xor_no_args_errors() {
    let result = call_err(&BoolModule, "xor", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn xor_one_arg_errors() {
    let result = call_err(&BoolModule, "xor", &[b(true)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn nand_no_args_errors() {
    let result = call_err(&BoolModule, "nand", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn nand_one_arg_errors() {
    let result = call_err(&BoolModule, "nand", &[b(true)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn nor_no_args_errors() {
    let result = call_err(&BoolModule, "nor", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn nor_one_arg_errors() {
    let result = call_err(&BoolModule, "nor", &[b(true)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn implies_no_args_errors() {
    let result = call_err(&BoolModule, "implies", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn implies_one_arg_errors() {
    let result = call_err(&BoolModule, "implies", &[b(true)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn eq_no_args_errors() {
    let result = call_err(&BoolModule, "eq", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn eq_one_arg_errors() {
    let result = call_err(&BoolModule, "eq", &[b(true)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// Type mismatch errors for binary functions

#[test]
fn and_first_arg_non_bool_errors() {
    let result = call_err(&BoolModule, "and", &[n(1.0), b(true)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn and_second_arg_non_bool_errors() {
    let result = call_err(&BoolModule, "and", &[b(true), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn or_first_arg_non_bool_errors() {
    let result = call_err(&BoolModule, "or", &[n(1.0), b(true)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn or_second_arg_non_bool_errors() {
    let result = call_err(&BoolModule, "or", &[b(true), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn xor_first_arg_non_bool_errors() {
    let result = call_err(&BoolModule, "xor", &[n(1.0), b(true)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn xor_second_arg_non_bool_errors() {
    let result = call_err(&BoolModule, "xor", &[b(true), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn nand_first_arg_non_bool_errors() {
    let result = call_err(&BoolModule, "nand", &[n(1.0), b(true)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn nand_second_arg_non_bool_errors() {
    let result = call_err(&BoolModule, "nand", &[b(true), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn nor_first_arg_non_bool_errors() {
    let result = call_err(&BoolModule, "nor", &[n(1.0), b(true)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn nor_second_arg_non_bool_errors() {
    let result = call_err(&BoolModule, "nor", &[b(true), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn implies_first_arg_non_bool_errors() {
    let result = call_err(&BoolModule, "implies", &[n(1.0), b(true)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn implies_second_arg_non_bool_errors() {
    let result = call_err(&BoolModule, "implies", &[b(true), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn eq_first_arg_non_bool_errors() {
    let result = call_err(&BoolModule, "eq", &[n(1.0), b(true)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn eq_second_arg_non_bool_errors() {
    let result = call_err(&BoolModule, "eq", &[b(true), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// Not type mismatch and arg count

#[test]
fn not_non_bool_errors() {
    let result = call_err(&BoolModule, "not", &[n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn not_no_args_errors() {
    let result = call_err(&BoolModule, "not", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// Lazy function error paths

#[test]
fn lazy_if_too_few_args_errors() {
    let expr = Expr::call("bool.if", vec![Expr::value(b(true))]);
    let result = eval_expr_result(&expr);
    assert!(result.is_err());
}

#[test]
fn lazy_if_cond_non_bool_errors() {
    let expr = Expr::call(
        "bool.if",
        vec![
            Expr::value(n(1.0)),
            Expr::value(b(true)),
            Expr::value(b(false)),
        ],
    );
    let result = eval_expr_result(&expr);
    assert!(result.is_err());
}

#[test]
fn lazy_and_too_few_args_errors() {
    let expr = Expr::call("bool.and", vec![Expr::value(b(true))]);
    let result = eval_expr_result(&expr);
    assert!(result.is_err());
}

#[test]
fn lazy_and_first_arg_non_bool_errors() {
    let expr = Expr::call("bool.and", vec![Expr::value(n(1.0)), Expr::value(b(true))]);
    let result = eval_expr_result(&expr);
    assert!(result.is_err());
}

#[test]
fn lazy_and_second_arg_non_bool_errors() {
    let expr = Expr::call("bool.and", vec![Expr::value(b(true)), Expr::value(n(1.0))]);
    let result = eval_expr_result(&expr);
    assert!(result.is_err());
}

#[test]
fn lazy_or_too_few_args_errors() {
    let expr = Expr::call("bool.or", vec![Expr::value(b(true))]);
    let result = eval_expr_result(&expr);
    assert!(result.is_err());
}

#[test]
fn lazy_or_first_arg_non_bool_errors() {
    let expr = Expr::call("bool.or", vec![Expr::value(n(1.0)), Expr::value(b(true))]);
    let result = eval_expr_result(&expr);
    assert!(result.is_err());
}

#[test]
fn lazy_or_second_arg_non_bool_errors() {
    let expr = Expr::call("bool.or", vec![Expr::value(b(false)), Expr::value(n(1.0))]);
    let result = eval_expr_result(&expr);
    assert!(result.is_err());
}

#[test]
fn lazy_if_false_branch_evaluated() {
    let expr = Expr::call(
        "bool.if",
        vec![
            Expr::value(b(false)),
            Expr::call("nonexistent", vec![]),
            Expr::value(b(true)),
        ],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn lazy_if_true_branch_evaluated() {
    let expr = Expr::call(
        "bool.if",
        vec![
            Expr::value(b(true)),
            Expr::value(b(true)),
            Expr::call("nonexistent", vec![]),
        ],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn lazy_and_short_circuits_returns_false() {
    let expr = Expr::call(
        "bool.and",
        vec![Expr::value(b(false)), Expr::call("nonexistent", vec![])],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn lazy_or_short_circuits_returns_true() {
    let expr = Expr::call(
        "bool.or",
        vec![Expr::value(b(true)), Expr::call("nonexistent", vec![])],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn lazy_and_both_false_returns_false() {
    let expr = Expr::call(
        "bool.and",
        vec![Expr::value(b(false)), Expr::value(b(false))],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn lazy_or_both_false_returns_false() {
    let expr = Expr::call(
        "bool.or",
        vec![Expr::value(b(false)), Expr::value(b(false))],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_bool(), Some(false));
}
