use handl_vm_backend_rust_interpreter::{CanonicalContext, Context, Host, Vm};
use handl_vm_common::{Module, ModuleHost};
use handl_vm_values_json::NonFunctionValue;

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
        "exponential",
        "poisson",
        "binomial",
        "perlin_1d",
        "perlin_2d",
        "perlin_3d",
        "simplex_2d",
        "simplex_3d",
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

fn eval_expr(expr: &handl_ast_canonical::Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(RandModule);
    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_int() {
    let expr = handl_ast_canonical::Expr::call(
        "rand.int",
        vec![
            handl_ast_canonical::Expr::value(n(1.0)),
            handl_ast_canonical::Expr::value(n(10.0)),
        ],
    );
    let result = eval_expr(&expr);
    let val = extract_num(&result) as i64;
    assert!((1..=10).contains(&val));
}

#[test]
fn vm_int_single_arg() {
    let expr =
        handl_ast_canonical::Expr::call("rand.int", vec![handl_ast_canonical::Expr::value(n(10.0))]);
    let result = eval_expr(&expr);
    let val = extract_num(&result) as i64;
    assert!((0..=10).contains(&val));
}

#[test]
fn vm_uuid() {
    let expr = handl_ast_canonical::Expr::call("rand.uuid", vec![]);
    let result = eval_expr(&expr);
    let uuid = extract_str(&result);
    assert_eq!(uuid.len(), 36);
}

#[test]
fn vm_gauss() {
    let expr = handl_ast_canonical::Expr::call(
        "rand.gauss",
        vec![
            handl_ast_canonical::Expr::value(n(0.0)),
            handl_ast_canonical::Expr::value(n(1.0)),
        ],
    );
    let result = eval_expr(&expr);
    assert!(extract_num(&result).is_finite());
}

#[test]
fn vm_perlin_1d() {
    let expr = handl_ast_canonical::Expr::call(
        "rand.perlin_1d",
        vec![handl_ast_canonical::Expr::value(n(0.5))],
    );
    let result = eval_expr(&expr);
    assert!(extract_num(&result).is_finite());
}

#[test]
fn vm_perlin_2d() {
    let expr = handl_ast_canonical::Expr::call(
        "rand.perlin_2d",
        vec![
            handl_ast_canonical::Expr::value(n(0.5)),
            handl_ast_canonical::Expr::value(n(0.5)),
        ],
    );
    let result = eval_expr(&expr);
    assert!(extract_num(&result).is_finite());
}

#[test]
fn vm_perlin_3d() {
    let expr = handl_ast_canonical::Expr::call(
        "rand.perlin_3d",
        vec![
            handl_ast_canonical::Expr::value(n(0.1)),
            handl_ast_canonical::Expr::value(n(0.2)),
            handl_ast_canonical::Expr::value(n(0.3)),
        ],
    );
    let result = eval_expr(&expr);
    assert!(extract_num(&result).is_finite());
}

#[test]
fn vm_seeded_int() {
    let expr = handl_ast_canonical::Expr::call(
        "rand.seeded_int",
        vec![
            handl_ast_canonical::Expr::value(n(42.0)),
            handl_ast_canonical::Expr::value(n(1.0)),
            handl_ast_canonical::Expr::value(n(100.0)),
        ],
    );
    let result = eval_expr(&expr);
    let val = extract_num(&result) as i64;
    assert!((1..=100).contains(&val));
}

#[test]
fn vm_seeded_shuffle() {
    let expr = handl_ast_canonical::Expr::call(
        "rand.seeded_shuffle",
        vec![
            handl_ast_canonical::Expr::value(n(42.0)),
            handl_ast_canonical::Expr::value(arr(&[
                serde_json::json!(1),
                serde_json::json!(2),
                serde_json::json!(3),
            ])),
        ],
    );
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
    assert!(host.function("rand.exponential").is_some());
    assert!(host.function("rand.poisson").is_some());
    assert!(host.function("rand.binomial").is_some());
    assert!(host.function("rand.perlin_1d").is_some());
    assert!(host.function("rand.perlin_2d").is_some());
    assert!(host.function("rand.perlin_3d").is_some());
    assert!(host.function("rand.simplex_2d").is_some());
    assert!(host.function("rand.simplex_3d").is_some());
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
