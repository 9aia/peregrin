//! Integration tests for the driver-lang pipeline (type-checked phase chains).
//!
//! These tests exercise `DriverPipeline` backed by `driver-lang`,
//! which enforces at compile time that phases connect correctly.

use ac_compiler::{DriverError, DriverPipeline, ExtractedDoc, Session, Source, Stage};
use ac_compiler_canonical::{default_cli_config, full_pipeline, typecheck_only_config, typecheck_pipeline, typegen_pipeline};
use ac_compiler_pass_extract::ExtractStage;
use ac_compiler_pass_generate::GenerateFromResolvedStage;
use ac_compiler_pass_parse::ParseStage;
use ac_compiler_pass_resolve::ResolveStage;
use ac_compiler_pass_typecheck::TypecheckOnlyStage;

fn default_config() -> ac_compiler::Config {
    default_cli_config()
}

fn typecheck_config() -> ac_compiler::Config {
    typecheck_only_config()
}

// --- typegen pipeline ---

#[test]
fn typegen_pipeline_generates_json_schema() {
    let yaml = r#"
$:
  host:
    type: string
  port:
    type: number

host: example.com
port: 8080
"#;
    let source = Source::new(yaml, "yaml");

    let mut driver = typegen_pipeline();
    let mut session = Session::new(default_config());
    let output = driver.run(source, &mut session).unwrap();

    assert_eq!(output["type"], "object");
    assert_eq!(output["properties"]["host"]["type"], "string");
    assert_eq!(output["properties"]["port"]["type"], "number");
}

#[test]
fn typegen_pipeline_invalid_format_returns_error() {
    let source = Source::new("not valid yaml: : :", "yaml");
    let mut driver = typegen_pipeline();
    let mut session = Session::new(default_config());
    let err = driver.run(source, &mut session).unwrap_err();
    assert_eq!(err.stage(), "parse");
}

#[test]
fn typegen_pipeline_no_fields_returns_error() {
    let yaml = r#"
just: some-data
"#;
    let source = Source::new(yaml, "yaml");
    let mut driver = typegen_pipeline();
    let mut session = Session::new(default_config());
    let err = driver.run(source, &mut session).unwrap_err();
    assert_eq!(err.stage(), "extract");
}

// --- typecheck pipeline ---

#[test]
fn typecheck_pipeline_valid_fields_no_errors() {
    let yaml = r#"
$:
  host:
    type: string
  port:
    type: number

host: example.com
port: 8080
"#;
    let source = Source::new(yaml, "yaml");

    let mut driver = typecheck_pipeline();
    let mut session = Session::new(typecheck_config());
    let result = driver.run(source, &mut session).unwrap();

    assert!(result.type_errors.is_empty());
}

#[test]
fn typecheck_pipeline_type_mismatch_reports_error() {
    let yaml = r#"
$:
  _base:
    type: string
    merge: true
  host:
    type: string

_base: hello
host: example.com
"#;
    let source = Source::new(yaml, "yaml");

    let mut driver = typecheck_pipeline();
    let mut session = Session::new(typecheck_config());
    let result = driver.run(source, &mut session).unwrap();

    assert!(!result.type_errors.is_empty());
}

// --- full pipeline ---

#[test]
fn full_pipeline_generates_with_typecheck() {
    let yaml = r#"
$:
  host:
    type: string
  port:
    type: number

host: example.com
port: 8080
"#;
    let source = Source::new(yaml, "yaml");

    let mut driver = full_pipeline();
    let mut session = Session::new(default_config());
    let output = driver.run(source, &mut session).unwrap();

    assert_eq!(output["type"], "object");
    assert_eq!(output["properties"]["host"]["type"], "string");
    assert_eq!(output["properties"]["port"]["type"], "number");
}

// --- Custom stage composition ---

#[test]
fn custom_stage_inserted_between_phases() {
    struct CountFieldsStage;
    impl Stage<ac_compiler::Config> for CountFieldsStage {
        type Input = ExtractedDoc;
        type Output = ExtractedDoc;
        fn name(&self) -> &'static str {
            "count-fields"
        }
        fn run(
            &mut self,
            input: ExtractedDoc,
            _session: &mut Session<ac_compiler::Config>,
        ) -> Result<ExtractedDoc, DriverError> {
            assert!(!input.extracted.schemas.is_empty());
            Ok(input)
        }
    }

    let yaml = r#"
$:
  host:
    type: string

host: example.com
"#;
    let source = Source::new(yaml, "yaml");

    let mut driver = DriverPipeline::new(ParseStage)
        .then(ExtractStage)
        .then(CountFieldsStage)
        .then(ResolveStage)
        .then(GenerateFromResolvedStage);

    let mut session = Session::new(default_config());
    let output = driver.run(source, &mut session).unwrap();

    assert_eq!(output["type"], "object");
    assert_eq!(output["properties"]["host"]["type"], "string");
}

// --- pass-lang optimization layer (still available for custom passes) ---

#[test]
fn pass_manager_runs_optimization_passes() {
    use pass_lang::{Pass as OptPass, PassError as OptPassError, PassManager};
    use pass_lang::Outcome;

    struct SquaredFieldsCount {
        count: usize}
    impl OptPass<Vec<String>> for SquaredFieldsCount {
        fn name(&self) -> &'static str {
            "count-squared"
        }
        fn run(&mut self, unit: &mut Vec<String>) -> Result<Outcome, OptPassError> {
            let before = unit.len();
            unit.retain(|s| !s.is_empty());
            Ok(Outcome::from_changed(unit.len() != before))
        }
    }

    let mut pm = PassManager::new();
    pm.add(SquaredFieldsCount { count: 0 });

    let mut unit = vec!["host".to_string(), "".to_string(), "port".to_string()];
    let report = pm.run(&mut unit).unwrap();

    assert_eq!(unit, vec!["host", "port"]);
    assert_eq!(report.changes(), 1);
    assert!(!report.converged()); // changed in this sweep

    // Run again — nothing to remove, so it converges
    let report2 = pm.run(&mut unit).unwrap();
    assert_eq!(report2.changes(), 0);
    assert!(report2.converged());
}

// --- Compile-time type safety ---

#[test]
fn driver_pipeline_type_checks_at_compile_time() {
    // This test exists to verify that the type-checked pipeline compiles.
    // If the stages were wired incorrectly (e.g. GenerateStage after ParseStage),
    // the code would not compile. The fact that this function compiles is the test.

    let _typegen = typegen_pipeline();
    let _typecheck = typecheck_pipeline();
    let _full = full_pipeline();

    // Also verify custom composition compiles
    let _custom = DriverPipeline::new(ParseStage)
        .then(ExtractStage)
        .then(TypecheckOnlyStage);
}
