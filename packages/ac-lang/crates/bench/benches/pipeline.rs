//! End-to-end pipeline benchmarks: parse → extract → typecheck.
//!
//! Measures the full typecheck pipeline at varying document sizes and
//! feature mixes, as well as individual pass costs.

use ac_bench::*;
use ac_compiler_core::{Compilation, Config, Pass, Source};
use ac_compiler_pass_extract::ExtractPass;
use ac_compiler_pass_parse::ParsePass;
use ac_compiler_pass_typecheck::TypecheckPass;
use ac_module_resolver_virtual::VirtualResolver;
use ac_typecheck_schema_json::JsonSchemaInterpreter;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

fn typecheck_config() -> Config {
    Config {
        dialect: Box::new(ac_dialect_canonical::AC_DIALECT),
        resolver: Box::new(VirtualResolver::new()),
        interpreter: Box::new(JsonSchemaInterpreter),
        backend: None,
        key_handler: Box::new(ac_typegen_key_set_special::SpecialKeyHandler),
        import_resolver: Box::new(ac_typegen_import::DefaultImportResolver),
        extensions: Default::default()}
}

fn run_pipeline(yaml: &str) {
    let source = Source::new(yaml, "yaml");
    let mut compilation = Compilation::new(source);
    let config = typecheck_config();
    ParsePass.run(&mut compilation, &config).unwrap();
    ExtractPass.run(&mut compilation, &config).unwrap();
    TypecheckPass.run(&mut compilation, &config).unwrap();
    compilation.resolve_error_locations();
    black_box(compilation);
}

fn run_parse_only(yaml: &str) {
    let source = Source::new(yaml, "yaml");
    let mut compilation = Compilation::new(source);
    let config = typecheck_config();
    ParsePass.run(&mut compilation, &config).unwrap();
    black_box(compilation);
}

fn run_parse_extract(yaml: &str) {
    let source = Source::new(yaml, "yaml");
    let mut compilation = Compilation::new(source);
    let config = typecheck_config();
    ParsePass.run(&mut compilation, &config).unwrap();
    ExtractPass.run(&mut compilation, &config).unwrap();
    black_box(compilation);
}

/// Full pipeline: parse → extract → typecheck on flat documents.
fn bench_flat_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline/flat");
    for n in [10, 50, 100, 500, 1000] {
        let yaml = flat_yaml(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &yaml, |b, yaml| {
            b.iter(|| run_pipeline(yaml));
        });
    }
    group.finish();
}

/// Full pipeline on documents with `expr:` on every field.
fn bench_expr_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline/expr");
    for n in [10, 50, 100, 500, 1000] {
        let yaml = expr_yaml(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &yaml, |b, yaml| {
            b.iter(|| run_pipeline(yaml));
        });
    }
    group.finish();
}

/// Full pipeline on documents with inheritable fields and nested data.
fn bench_inherit_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline/inherit");
    for (n, depth) in [(10, 5), (20, 10), (50, 5), (50, 20)] {
        let yaml = inherit_yaml(n, depth);
        group.bench_with_input(
            BenchmarkId::new(format!("{n}x{depth}"), format!("n={n},d={depth}")),
            &yaml,
            |b, yaml| {
                b.iter(|| run_pipeline(yaml));
            },
        );
    }
    group.finish();
}

/// Full pipeline on mixed-feature documents.
fn bench_mixed_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline/mixed");
    for n in [10, 50, 100, 500] {
        let yaml = mixed_yaml(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &yaml, |b, yaml| {
            b.iter(|| run_pipeline(yaml));
        });
    }
    group.finish();
}

/// Individual pass cost: parse only.
fn bench_parse_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline/parse_only");
    for n in [100, 500, 1000] {
        let yaml = flat_yaml(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &yaml, |b, yaml| {
            b.iter(|| run_parse_only(yaml));
        });
    }
    group.finish();
}

/// Individual pass cost: parse + extract (no typecheck).
fn bench_parse_extract(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline/parse_extract");
    for n in [100, 500, 1000] {
        let yaml = flat_yaml(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &yaml, |b, yaml| {
            b.iter(|| run_parse_extract(yaml));
        });
    }
    group.finish();
}

criterion_group!(
    pipeline_benches,
    bench_flat_pipeline,
    bench_expr_pipeline,
    bench_inherit_pipeline,
    bench_mixed_pipeline,
    bench_parse_only,
    bench_parse_extract,
);
criterion_main!(pipeline_benches);
