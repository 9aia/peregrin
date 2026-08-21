use ac_runtime::NonFunctionValue;
use ac_runtime_core::{Module, ModuleHost};
use ac_vm::{Context, Host, Vm};

use super::module::RandModule;
use super::test_helpers::*;

// ── module structure ─────────────────────────────────────────────

#[test]
fn module_name() {
    assert_eq!(RandModule.name(), "rand");
}

#[test]
fn all_functions_registered() {
    let names = [
        "int",
        "float",
        "bool",
        "pick",
        "choice",
        "sample",
        "shuffle",
        "weighted_pick",
        "string",
        "bytes",
        "uuid",
        "gauss",
        "perlin_1d",
        "perlin_2d",
        "perlin_3d",
        "seeded_int",
        "seeded_float",
        "seeded_bool",
        "seeded_pick",
        "seeded_choice",
        "seeded_sample",
        "seeded_shuffle",
        "seeded_string",
        "seeded_gauss",
    ];
    for name in &names {
        assert!(
            RandModule.function(name).is_some(),
            "function '{name}' should be registered"
        );
    }
}

#[test]
fn unknown_function_returns_none() {
    assert!(RandModule.function("nonexistent").is_none());
}

// ── VM integration ───────────────────────────────────────────────

fn eval_expr(expr: &ac_ast::Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(RandModule);
    let vm = Vm::new(host);
    let mut ctx = Context::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_int() {
    let expr = ac_ast::Expr::call("rand.int", vec![ac_ast::Expr::value(n(1.0)), ac_ast::Expr::value(n(10.0))]);
    let result = eval_expr(&expr);
    let val = extract_num(&result) as i64;
    assert!((1..=10).contains(&val));
}

#[test]
fn vm_int_single_arg() {
    let expr = ac_ast::Expr::call("rand.int", vec![ac_ast::Expr::value(n(10.0))]);
    let result = eval_expr(&expr);
    let val = extract_num(&result) as i64;
    assert!((0..=10).contains(&val));
}

#[test]
fn vm_uuid() {
    let expr = ac_ast::Expr::call("rand.uuid", vec![]);
    let result = eval_expr(&expr);
    let uuid = extract_str(&result);
    assert_eq!(uuid.len(), 36);
}

#[test]
fn vm_gauss() {
    let expr = ac_ast::Expr::call("rand.gauss", vec![ac_ast::Expr::value(n(0.0)), ac_ast::Expr::value(n(1.0))]);
    let result = eval_expr(&expr);
    assert!(extract_num(&result).is_finite());
}

#[test]
fn vm_perlin_1d() {
    let expr = ac_ast::Expr::call("rand.perlin_1d", vec![ac_ast::Expr::value(n(0.5))]);
    let result = eval_expr(&expr);
    assert!(extract_num(&result).is_finite());
}

#[test]
fn vm_perlin_2d() {
    let expr = ac_ast::Expr::call("rand.perlin_2d", vec![ac_ast::Expr::value(n(0.5)), ac_ast::Expr::value(n(0.5))]);
    let result = eval_expr(&expr);
    assert!(extract_num(&result).is_finite());
}

#[test]
fn vm_perlin_3d() {
    let expr = ac_ast::Expr::call("rand.perlin_3d", vec![ac_ast::Expr::value(n(0.1)), ac_ast::Expr::value(n(0.2)), ac_ast::Expr::value(n(0.3))]);
    let result = eval_expr(&expr);
    assert!(extract_num(&result).is_finite());
}

#[test]
fn vm_seeded_int() {
    let expr = ac_ast::Expr::call("rand.seeded_int", vec![ac_ast::Expr::value(n(42.0)), ac_ast::Expr::value(n(1.0)), ac_ast::Expr::value(n(100.0))]);
    let result = eval_expr(&expr);
    let val = extract_num(&result) as i64;
    assert!((1..=100).contains(&val));
}

#[test]
fn vm_seeded_shuffle() {
    let expr = ac_ast::Expr::call("rand.seeded_shuffle", vec![ac_ast::Expr::value(n(42.0)), ac_ast::Expr::value(arr(&[serde_json::json!(1), serde_json::json!(2), serde_json::json!(3)]))]);
    let result = eval_expr(&expr);
    assert_eq!(extract_array(&result).len(), 3);
}

#[test]
fn vm_resolves_via_module_host() {
    let host = ModuleHost::new().with(RandModule);
    assert!(host.function("rand.int").is_some());
    assert!(host.function("rand.float").is_some());
    assert!(host.function("rand.bool").is_some());
    assert!(host.function("rand.pick").is_some());
    assert!(host.function("rand.choice").is_some());
    assert!(host.function("rand.sample").is_some());
    assert!(host.function("rand.shuffle").is_some());
    assert!(host.function("rand.weighted_pick").is_some());
    assert!(host.function("rand.string").is_some());
    assert!(host.function("rand.bytes").is_some());
    assert!(host.function("rand.uuid").is_some());
    assert!(host.function("rand.gauss").is_some());
    assert!(host.function("rand.perlin_1d").is_some());
    assert!(host.function("rand.perlin_2d").is_some());
    assert!(host.function("rand.perlin_3d").is_some());
    assert!(host.function("rand.seeded_int").is_some());
    assert!(host.function("rand.seeded_float").is_some());
    assert!(host.function("rand.seeded_bool").is_some());
    assert!(host.function("rand.seeded_pick").is_some());
    assert!(host.function("rand.seeded_choice").is_some());
    assert!(host.function("rand.seeded_sample").is_some());
    assert!(host.function("rand.seeded_shuffle").is_some());
    assert!(host.function("rand.seeded_string").is_some());
    assert!(host.function("rand.seeded_gauss").is_some());
}
