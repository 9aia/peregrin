use crate::{Compilation, Source};

#[test]
fn source_new_without_path() {
    let source = Source::new("name: ac", "yaml");
    assert_eq!(source.input, "name: ac");
    assert_eq!(source.format, "yaml");
    assert!(source.path.is_none());
}

#[test]
fn source_with_path_sets_path() {
    let source = Source::new("name: ac", "yaml").with_path("/tmp/test.yaml");
    assert_eq!(
        source.path,
        Some(std::path::PathBuf::from("/tmp/test.yaml"))
    );
}

#[test]
fn source_base_path_extracts_parent() {
    let source = Source::new("name: ac", "yaml").with_path("/tmp/dir/test.yaml");
    assert_eq!(
        source.base_path(),
        Some(std::path::PathBuf::from("/tmp/dir"))
    );
}

#[test]
fn source_base_path_none_without_path() {
    let source = Source::new("name: ac", "yaml");
    assert!(source.base_path().is_none());
}

#[test]
fn source_base_path_none_for_root_file() {
    let source = Source::new("name: ac", "yaml").with_path("/test.yaml");
    // parent of /test.yaml is /, which is Some("/")
    assert!(source.base_path().is_some());
}

#[test]
fn compilation_new_starts_empty() {
    let source = Source::new("name: ac", "yaml");
    let compilation = Compilation::new(source);

    assert!(compilation.parsed.is_none());
    assert!(compilation.extracted.is_none());
    assert!(compilation.typegen_ctx.is_none());
    assert!(compilation.type_errors.is_empty());
    assert!(compilation.output.is_none());
}

#[test]
fn compilation_parsed_helper_returns_ref() {
    let source = Source::new("name: ac", "yaml");
    let mut compilation = Compilation::new(source);
    compilation.parsed = Some(ac_runtime::RuntimeValue::new(serde_json::json!({"a": 1})));

    let parsed = compilation.parsed().expect("parsed should be set");
    assert_eq!(parsed.get("a").unwrap().as_i64(), Some(1));
}

#[test]
fn compilation_extracted_helper_returns_ref() {
    let source = Source::new("name: ac", "yaml");
    let compilation = Compilation::new(source);
    assert!(compilation.extracted().is_none());
}
