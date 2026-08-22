//! Expression inference benchmarks.
//!
//! Measures `ac_typecheck_infer` performance: parsing, inference, and
//! the effect of AST caching across repeated calls.

use ac_bench::*;
use ac_dialect_canonical::AC_DIALECT;
use ac_typecheck_core::{Type, TypeEnv};
use ac_typecheck_infer::{FunctionTable, ParseCache, infer, infer_cached, parse_expr};
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

/// `parse_expr` on literal expressions.
fn bench_parse_literals(c: &mut Criterion) {
    let mut group = c.benchmark_group("infer/parse_literal");
    for n in [10, 100, 1000] {
        let exprs: Vec<String> = (0..n).map(|i| literal_expr(i)).collect();
        group.bench_with_input(BenchmarkId::from_parameter(n), &exprs, |b, exprs| {
            b.iter(|| {
                for e in exprs {
                    black_box(parse_expr(e, &AC_DIALECT));
                }
            });
        });
    }
    group.finish();
}

/// `parse_expr` on nested function call expressions.
fn bench_parse_nested_calls(c: &mut Criterion) {
    let mut group = c.benchmark_group("infer/parse_nested_call");
    for depth in [1, 3, 5, 10, 20] {
        let expr = nested_call_expr(depth);
        group.bench_with_input(BenchmarkId::from_parameter(depth), &expr, |b, expr| {
            b.iter(|| black_box(parse_expr(black_box(expr), &AC_DIALECT)));
        });
    }
    group.finish();
}

/// `infer` (parse + infer) on a batch of mixed expressions.
fn bench_infer_batch(c: &mut Criterion) {
    let env = TypeEnv::new();
    let functions = FunctionTable::new();
    let mut group = c.benchmark_group("infer/infer_batch");
    for n in [10, 50, 100, 500] {
        let exprs = mixed_exprs(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &exprs, |b, exprs| {
            b.iter(|| {
                for e in exprs {
                    black_box(infer(
                        black_box(e),
                        black_box(&env),
                        black_box(&functions),
                        &AC_DIALECT,
                    ));
                }
            });
        });
    }
    group.finish();
}

/// `infer_cached` — measures cache hit speedup on repeated expressions.
fn bench_infer_cached(c: &mut Criterion) {
    let env = TypeEnv::new();
    let functions = FunctionTable::new();
    let mut group = c.benchmark_group("infer/infer_cached");
    for (n, repeats) in [(100, 1), (100, 5), (100, 10)] {
        let exprs = mixed_exprs(n);
        let label = format!("{n}x{repeats}");
        group.bench_with_input(
            BenchmarkId::from_parameter(&label),
            &(exprs, repeats),
            |b, (exprs, repeats)| {
                b.iter(|| {
                    let mut cache: ParseCache = std::collections::HashMap::new();
                    for _ in 0..*repeats {
                        for e in exprs {
                            black_box(infer_cached(
                                black_box(e),
                                black_box(&env),
                                black_box(&functions),
                                black_box(&mut cache),
                                &AC_DIALECT,
                            ));
                        }
                    }
                });
            },
        );
    }
    group.finish();
}

/// `infer` on deeply nested function calls — measures recursive inference depth.
fn bench_infer_nested_calls(c: &mut Criterion) {
    let env = TypeEnv::new();
    let mut functions = FunctionTable::new();
    functions.insert(
        "add",
        ac_typecheck_core::FunctionType {
            type_params: vec![],
            params: vec![
                ("a".to_string(), Type::Number),
                ("b".to_string(), Type::Number),
            ],
            return_type: Box::new(Type::Number),
        },
    );
    let mut group = c.benchmark_group("infer/infer_nested_call");
    for depth in [1, 3, 5, 10, 20] {
        let expr = nested_call_expr(depth);
        group.bench_with_input(BenchmarkId::from_parameter(depth), &expr, |b, expr| {
            b.iter(|| {
                black_box(infer(
                    black_box(expr),
                    black_box(&env),
                    black_box(&functions),
                    &AC_DIALECT,
                ));
            });
        });
    }
    group.finish();
}

/// `infer` on generic function calls — measures TraitRegistry + unification hot path.
fn bench_infer_generic_calls(c: &mut Criterion) {
    let env = TypeEnv::new();
    let functions = FunctionTable::from_signatures(
        &ac_typecheck_infer::HostFunctionSigs::default_host().0,
        &ac_typecheck_schema_json::JsonSchemaInterpreter,
    );
    let mut group = c.benchmark_group("infer/generic_calls");
    for n in [10, 50, 100, 500] {
        let exprs = generic_call_exprs(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &exprs, |b, exprs| {
            b.iter(|| {
                for e in exprs {
                    black_box(infer(
                        black_box(e),
                        black_box(&env),
                        black_box(&functions),
                        &AC_DIALECT,
                    ));
                }
            });
        });
    }
    group.finish();
}

criterion_group!(
    infer_benches,
    bench_parse_literals,
    bench_parse_nested_calls,
    bench_infer_batch,
    bench_infer_cached,
    bench_infer_nested_calls,
    bench_infer_generic_calls,
);
criterion_main!(infer_benches);
