use crate::{Compilation, Config, Pass, Pipeline, Source};

/// A no-op pass that does nothing — for testing the pipeline mechanics.
struct NoopPass;

impl Pass for NoopPass {
    fn run(&self, _compilation: &mut Compilation, _config: &Config) -> Result<(), String> {
        Ok(())
    }
}

/// A pass that sets output to verify it ran.
struct SetOutputPass;

impl Pass for SetOutputPass {
    fn run(&self, compilation: &mut Compilation, _config: &Config) -> Result<(), String> {
        compilation.output = Some(serde_json::json!({"pass": "set"}));
        Ok(())
    }
}

/// A pass that always fails.
struct FailingPass;

impl Pass for FailingPass {
    fn run(&self, _compilation: &mut Compilation, _config: &Config) -> Result<(), String> {
        Err("intentional failure".to_string())
    }
}

fn test_config() -> Config {
    Config {
        dialect: Box::new(ac_dialect_canonical::AC_DIALECT),
        resolver: Box::new(ac_module_resolver_virtual::VirtualResolver::new()),
        interpreter: Box::new(ac_typecheck_schema_json::JsonSchemaInterpreter),
        backend: None,
        key_handler: Box::new(ac_typegen_key_set_special::SpecialKeyHandler),
        import_resolver: Box::new(ac_typegen_import::DefaultImportResolver),
    }
}

fn test_config_with_backend() -> Config {
    Config {
        dialect: Box::new(ac_dialect_canonical::AC_DIALECT),
        resolver: Box::new(ac_module_resolver_virtual::VirtualResolver::new()),
        interpreter: Box::new(ac_typecheck_schema_json::JsonSchemaInterpreter),
        backend: Some(Box::new(
            ac_typegen_backend_json_schema::JsonSchemaBackend,
        )),
        key_handler: Box::new(ac_typegen_key_set_special::SpecialKeyHandler),
        import_resolver: Box::new(ac_typegen_import::DefaultImportResolver),
    }
}

#[test]
fn pipeline_new_starts_empty() {
    let pipeline = Pipeline::new(test_config());
    assert!(pipeline.passes.is_empty());
}

#[test]
fn pipeline_with_pass_adds_pass() {
    let pipeline = Pipeline::new(test_config()).with_pass(NoopPass);
    assert_eq!(pipeline.passes.len(), 1);
}

#[test]
fn pipeline_with_multiple_passes() {
    let pipeline = Pipeline::new(test_config())
        .with_pass(NoopPass)
        .with_pass(SetOutputPass)
        .with_pass(NoopPass);
    assert_eq!(pipeline.passes.len(), 3);
}

#[test]
fn pipeline_run_executes_passes_in_order() {
    let pipeline = Pipeline::new(test_config()).with_pass(SetOutputPass);
    let source = Source::new("name: ac", "yaml");
    let compilation = pipeline.run(source).expect("pipeline should succeed");
    assert_eq!(compilation.output, Some(serde_json::json!({"pass": "set"})));
}

#[test]
fn pipeline_run_empty_pipeline_succeeds() {
    let pipeline = Pipeline::new(test_config());
    let source = Source::new("name: ac", "yaml");
    let compilation = pipeline.run(source).expect("pipeline should succeed");
    assert!(compilation.output.is_none());
}

#[test]
fn pipeline_run_failing_pass_propagates_error() {
    let pipeline = Pipeline::new(test_config()).with_pass(FailingPass);
    let source = Source::new("name: ac", "yaml");
    let result = pipeline.run(source);
    match result {
        Err(e) => assert_eq!(e, "intentional failure"),
        Ok(_) => panic!("expected error"),
    }
}

#[test]
fn pipeline_run_failing_pass_stops_subsequent_passes() {
    let pipeline = Pipeline::new(test_config())
        .with_pass(FailingPass)
        .with_pass(SetOutputPass);
    let source = Source::new("name: ac", "yaml");
    let result = pipeline.run(source);
    match result {
        Err(e) => assert_eq!(e, "intentional failure"),
        Ok(_) => panic!("expected error"),
    }
}

#[test]
fn pipeline_run_preserves_source() {
    let pipeline = Pipeline::new(test_config()).with_pass(NoopPass);
    let source = Source::new("name: ac", "yaml").with_path("/tmp/test.yaml");
    let compilation = pipeline.run(source).expect("pipeline should succeed");
    assert_eq!(compilation.source.input, "name: ac");
    assert_eq!(compilation.source.format, "yaml");
    assert_eq!(
        compilation.source.path,
        Some(std::path::PathBuf::from("/tmp/test.yaml"))
    );
}

#[test]
fn config_with_backend() {
    let config = test_config_with_backend();
    assert!(config.backend.is_some());
}

#[test]
fn config_without_backend() {
    let config = test_config();
    assert!(config.backend.is_none());
}
