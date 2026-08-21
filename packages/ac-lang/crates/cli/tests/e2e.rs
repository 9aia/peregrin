use std::path::PathBuf;
use std::process::Command;

fn cli_bin() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/cli/
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Workspace root is two levels up: crates/cli/ -> crates/ -> ac-lang/
    let workspace_root = manifest.parent().unwrap().parent().unwrap();
    workspace_root.join("target").join("debug").join("ac")
}

fn test_data(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_data")
        .join(file)
}

fn run_cli(args: &[&str]) -> (String, String, i32) {
    let bin = cli_bin();
    let output = Command::new(&bin)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

#[test]
fn parse_yaml() {
    let (stdout, _stderr, code) = run_cli(&[
        "parse",
        "-i",
        &test_data("config.yaml").to_string_lossy(),
        "-f",
        "yaml",
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("name"));
}

#[test]
fn validate_valid_file() {
    let (stdout, _stderr, code) = run_cli(&[
        "validate",
        "-i",
        &test_data("config.yaml").to_string_lossy(),
        "-f",
        "yaml",
        "-s",
        &test_data("schema.json").to_string_lossy(),
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("valid"));
}

#[test]
fn fields_self_describing() {
    let (stdout, _stderr, code) = run_cli(&[
        "fields",
        "-i",
        &test_data("self_describing.yml").to_string_lossy(),
        "-f",
        "yaml",
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("fields:"));
}

#[test]
fn fields_invalid() {
    let (_stdout, stderr, code) = run_cli(&[
        "fields",
        "-i",
        &test_data("self_describing_invalid.yml").to_string_lossy(),
        "-f",
        "yaml",
    ]);
    assert_eq!(code, 1);
    assert!(stderr.contains("INVALID") || stderr.contains("error"));
}

#[test]
fn fields_reserved_name() {
    let (_stdout, stderr, code) = run_cli(&[
        "fields",
        "-i",
        &test_data("reserved_field.yml").to_string_lossy(),
        "-f",
        "yaml",
    ]);
    assert_eq!(code, 1);
    assert!(stderr.contains("reserved"));
}

#[test]
fn fields_with_interpolation() {
    let (stdout, _stderr, code) = run_cli(&[
        "fields",
        "-i",
        &test_data("interpolation.yml").to_string_lossy(),
        "-f",
        "yaml",
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("fields:"));
}

#[test]
fn fields_with_computed() {
    let (stdout, _stderr, code) = run_cli(&[
        "fields",
        "-i",
        &test_data("computed.yml").to_string_lossy(),
        "-f",
        "yaml",
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("fields:"));
}

#[test]
fn fields_with_import() {
    let (stdout, _stderr, code) = run_cli(&[
        "fields",
        "-i",
        &test_data("import_main.yml").to_string_lossy(),
        "-f",
        "yaml",
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("fields:"));
}

#[test]
fn fields_with_merge() {
    let (stdout, _stderr, code) = run_cli(&[
        "fields",
        "-i",
        &test_data("merge_main.yml").to_string_lossy(),
        "-f",
        "yaml",
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("fields:"));
}

#[test]
fn fields_with_arg() {
    let (stdout, _stderr, code) = run_cli(&[
        "fields",
        "-i",
        &test_data("arg_basic.yml").to_string_lossy(),
        "-f",
        "yaml",
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("fields:"));
}

#[test]
fn fields_with_dynamic_import() {
    let (stdout, _stderr, code) = run_cli(&[
        "fields",
        "-i",
        &test_data("dynamic_import_main.yml").to_string_lossy(),
        "-f",
        "yaml",
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("fields:"));
}

#[test]
fn fields_with_import_functions() {
    let (stdout, _stderr, code) = run_cli(&[
        "fields",
        "-i",
        &test_data("import_functions_main.yml").to_string_lossy(),
        "-f",
        "yaml",
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("fields:"));
}

#[test]
fn fields_with_import_field() {
    let (stdout, _stderr, code) = run_cli(&[
        "fields",
        "-i",
        &test_data("import_field_main.yml").to_string_lossy(),
        "-f",
        "yaml",
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("fields:"));
}
