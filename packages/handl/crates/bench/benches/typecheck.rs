//! Typecheck sub-crate benchmarks.
//!
//! Isolates each typecheck key crate (expr, arg, merge, inherit, mutable)
//! to measure their individual throughput independently of the pipeline.

use handl_bench::*;
use handl_typecheck_core::TypeEnv;
use handl_typecheck_infer::FunctionTable;
use handl_typecheck_schema_json::JsonSchemaInterpreter;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

fn interpreter() -> JsonSchemaInterpreter {
    JsonSchemaInterpreter
}

/// `handl_typecheck_keys::expr::check` at varying field counts.
fn bench_key_expr(c: &mut Criterion) {
    let interp = interpreter();
    let mut group = c.benchmark_group("typecheck_key_expr/check");
    for n in [10, 50, 100, 500, 1000] {
        let schemas = expr_schemas(n);
        let env = TypeEnv::new();
        let functions = FunctionTable::new();
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                let errors = handl_typecheck_keys::expr::check(
                    black_box(&schemas),
                    black_box(&env),
                    black_box(&functions),
                    black_box(&interp),
                );
                black_box(errors);
            });
        });
    }
    group.finish();
}

/// `handl_typecheck_keys::arg::check` at varying field counts.
fn bench_key_arg(c: &mut Criterion) {
    let interp = interpreter();
    let mut group = c.benchmark_group("typecheck_key_arg/check");
    for n in [10, 50, 100, 500, 1000] {
        let schemas = arg_schemas(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                let errors = handl_typecheck_keys::arg::check(black_box(&schemas), black_box(&interp));
                black_box(errors);
            });
        });
    }
    group.finish();
}

/// `handl_typecheck_keys::merge::check` at varying field counts.
fn bench_key_merge(c: &mut Criterion) {
    let interp = interpreter();
    let mut group = c.benchmark_group("typecheck_key_merge/check");
    for n in [10, 50, 100, 500] {
        let schemas = merge_schemas(n);
        let env = TypeEnv::new();
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                let errors = handl_typecheck_keys::merge::check(
                    black_box(&schemas),
                    black_box(&interp),
                    black_box(&env),
                );
                black_box(errors);
            });
        });
    }
    group.finish();
}

/// `handl_typecheck_keys::inherit::check` at varying field counts and nesting depths.
fn bench_key_inherit(c: &mut Criterion) {
    let interp = interpreter();
    let mut group = c.benchmark_group("typecheck_key_inherit/check");
    for (n, depth) in [(10, 5), (20, 10), (50, 5), (50, 20), (100, 10)] {
        let schemas = inheritable_schemas(n);
        let data = nested_data(n, depth);
        group.bench_with_input(
            BenchmarkId::new(format!("{n}x{depth}"), format!("n={n},d={depth}")),
            &(n, depth),
            |b, _| {
                b.iter(|| {
                    let errors = handl_typecheck_keys::inherit::check(
                        black_box(&schemas),
                        black_box(&data),
                        black_box(&interp),
                    );
                    black_box(errors);
                });
            },
        );
    }
    group.finish();
}

/// `handl_typecheck_keys::mutable::check` at varying field counts and nesting depths.
fn bench_key_mutable(c: &mut Criterion) {
    let mut group = c.benchmark_group("typecheck_key_mutable/check");
    for (n, depth) in [(10, 5), (20, 10), (50, 5), (50, 20), (100, 10)] {
        let fields = immutable_fields(n);
        let data = nested_data_uniform(n, depth);
        group.bench_with_input(
            BenchmarkId::new(format!("{n}x{depth}"), format!("n={n},d={depth}")),
            &(n, depth),
            |b, _| {
                b.iter(|| {
                    let errors =
                        handl_typecheck_keys::mutable::check(black_box(&fields), black_box(&data));
                    black_box(errors);
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    typecheck_benches,
    bench_key_expr,
    bench_key_arg,
    bench_key_merge,
    bench_key_inherit,
    bench_key_mutable,
);
criterion_main!(typecheck_benches);
