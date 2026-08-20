//! Core typecheck benchmarks: schema interpretation and type compatibility.
//!
//! Measures `ac_typecheck_schema_json::interpret_schema` and
//! `ac_typecheck_core::is_compatible` in isolation.

use ac_bench::*;
use ac_typecheck_core::{is_compatible, SchemaInterpreter, Type};
use ac_typecheck_schema_json::{interpret_schema, JsonSchemaInterpreter};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

// ---------------------------------------------------------------------------
// Schema interpretation
// ---------------------------------------------------------------------------

/// `interpret_schema` on primitive types.
fn bench_interpret_primitive(c: &mut Criterion) {
    let mut group = c.benchmark_group("core/interpret_primitive");
    for n in [100, 1000, 10000] {
        let schemas: Vec<_> = (0..n).map(simple_schema).collect();
        group.bench_with_input(BenchmarkId::from_parameter(n), &schemas, |b, schemas| {
            b.iter(|| {
                for s in schemas {
                    black_box(interpret_schema(black_box(s)));
                }
            });
        });
    }
    group.finish();
}

/// `interpret_schema` on objects with varying property counts.
fn bench_interpret_object(c: &mut Criterion) {
    let mut group = c.benchmark_group("core/interpret_object");
    for n in [5, 10, 50, 100] {
        let schema = object_schema(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &schema, |b, schema| {
            b.iter(|| black_box(interpret_schema(black_box(schema))));
        });
    }
    group.finish();
}

/// `interpret_schema` on deeply nested objects.
fn bench_interpret_nested_object(c: &mut Criterion) {
    let mut group = c.benchmark_group("core/interpret_nested_object");
    for depth in [1, 5, 10, 20, 50] {
        let schema = nested_object_schema(depth);
        group.bench_with_input(BenchmarkId::from_parameter(depth), &schema, |b, schema| {
            b.iter(|| black_box(interpret_schema(black_box(schema))));
        });
    }
    group.finish();
}

/// `SchemaInterpreter::interpret` via trait object (measures dynamic dispatch overhead).
fn bench_interpret_trait_object(c: &mut Criterion) {
    let interp = JsonSchemaInterpreter;
    let mut group = c.benchmark_group("core/interpret_trait_object");
    for n in [100, 1000, 10000] {
        let schemas: Vec<_> = (0..n).map(simple_schema).collect();
        group.bench_with_input(BenchmarkId::from_parameter(n), &schemas, |b, schemas| {
            b.iter(|| {
                for s in schemas {
                    black_box(interp.interpret(black_box(s)));
                }
            });
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Type compatibility
// ---------------------------------------------------------------------------

/// `is_compatible` on primitive type pairs.
fn bench_compat_primitive(c: &mut Criterion) {
    let mut group = c.benchmark_group("core/compat_primitive");
    for n in [100, 1000, 10000] {
        let types: Vec<Type> = (0..n).map(simple_type).collect();
        group.bench_with_input(BenchmarkId::from_parameter(n), &types, |b, types| {
            b.iter(|| {
                for i in 0..types.len() {
                    let j = (i + 1) % types.len();
                    black_box(is_compatible(black_box(&types[i]), black_box(&types[j])));
                }
            });
        });
    }
    group.finish();
}

/// `is_compatible` on object types with varying property counts.
fn bench_compat_object(c: &mut Criterion) {
    let mut group = c.benchmark_group("core/compat_object");
    for n in [5, 10, 50, 100] {
        let a = object_type(n);
        let b = object_type(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &(a, b), |bencher, (a, b)| {
            bencher.iter(|| black_box(is_compatible(black_box(a), black_box(b))));
        });
    }
    group.finish();
}

/// `is_compatible` on deeply nested object types.
fn bench_compat_nested_object(c: &mut Criterion) {
    let mut group = c.benchmark_group("core/compat_nested_object");
    for depth in [1, 5, 10, 20, 50] {
        let a = nested_object_type(depth);
        let b = nested_object_type(depth);
        group.bench_with_input(BenchmarkId::from_parameter(depth), &(a, b), |bencher, (a, b)| {
            bencher.iter(|| black_box(is_compatible(black_box(a), black_box(b))));
        });
    }
    group.finish();
}

/// `is_compatible` on array types.
fn bench_compat_array(c: &mut Criterion) {
    let mut group = c.benchmark_group("core/compat_array");
    for n in [100, 1000, 10000] {
        let a = array_type(Type::Number);
        let b = array_type(Type::Number);
        group.bench_with_input(BenchmarkId::from_parameter(n), &(a, b), |bencher, (a, b)| {
            bencher.iter(|| {
                for _ in 0..n {
                    black_box(is_compatible(black_box(a), black_box(b)));
                }
            });
        });
    }
    group.finish();
}

criterion_group!(
    core_benches,
    bench_interpret_primitive,
    bench_interpret_object,
    bench_interpret_nested_object,
    bench_interpret_trait_object,
    bench_compat_primitive,
    bench_compat_object,
    bench_compat_nested_object,
    bench_compat_array,
);
criterion_main!(core_benches);
