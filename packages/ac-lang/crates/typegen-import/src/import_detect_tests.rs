use crate::import_detect::{detect_import_path, extract_data_path, ImportKind};

#[test]
fn detects_static_single_quotes() {
    let result = detect_import_path("import('./base.yml')");
    assert_eq!(result, ImportKind::Static("./base.yml".to_string()));
}

#[test]
fn detects_static_double_quotes() {
    let result = detect_import_path("import(\"./base.yml\")");
    assert_eq!(result, ImportKind::Static("./base.yml".to_string()));
}

#[test]
fn detects_data_driven() {
    let result = detect_import_path("import(arg)");
    assert_eq!(result, ImportKind::DataDriven);
}

#[test]
fn returns_none_for_non_import() {
    let result = detect_import_path("concat(a, b)");
    assert_eq!(result, ImportKind::None);
}

#[test]
fn returns_none_for_empty() {
    let result = detect_import_path("");
    assert_eq!(result, ImportKind::None);
}

#[test]
fn returns_none_for_dynamic_expression() {
    let result = detect_import_path("import(concat(arg, env.ENV))");
    assert_eq!(result, ImportKind::None);
}

#[test]
fn handles_whitespace() {
    let result = detect_import_path("  import('./base.yml')  ");
    assert_eq!(result, ImportKind::Static("./base.yml".to_string()));
}

#[test]
fn extracts_data_path_from_string_field() {
    let data = serde_json::json!({ "import": "./base.yml" });
    let result = extract_data_path(&data, "import");
    assert_eq!(result, Some("./base.yml".to_string()));
}

#[test]
fn returns_none_for_non_string_field() {
    let data = serde_json::json!({ "import": 42 });
    let result = extract_data_path(&data, "import");
    assert_eq!(result, None);
}

#[test]
fn returns_none_for_missing_field() {
    let data = serde_json::json!({});
    let result = extract_data_path(&data, "import");
    assert_eq!(result, None);
}
