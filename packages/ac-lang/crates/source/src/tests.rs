use super::*;

#[test]
fn resolve_root_path_returns_1_1() {
    let map = SourceMap::new("key: value\n");
    assert_eq!(map.resolve_path(""), Some(SourceLocation::new(1, 1)));
}

#[test]
fn resolve_top_level_key_in_yaml() {
    let source = "server:\n  port: 8080\n  host: localhost\n";
    let map = SourceMap::new(source);
    assert_eq!(map.resolve_path("server"), Some(SourceLocation::new(1, 1)));
}

#[test]
fn resolve_nested_key_in_yaml() {
    let source = "server:\n  port: 8080\n  host: localhost\n";
    let map = SourceMap::new(source);
    assert_eq!(
        map.resolve_path("server.port"),
        Some(SourceLocation::new(2, 3))
    );
}

#[test]
fn resolve_deeply_nested_key_in_yaml() {
    let source = "config:\n  server:\n    port: 8080\n    host: localhost\n";
    let map = SourceMap::new(source);
    assert_eq!(
        map.resolve_path("config.server.port"),
        Some(SourceLocation::new(3, 5))
    );
}

#[test]
fn resolve_key_in_json() {
    let source = "{\n  \"server\": {\n    \"port\": 8080\n  }\n}\n";
    let map = SourceMap::new(source);
    assert_eq!(
        map.resolve_path("server.port"),
        Some(SourceLocation::new(3, 5))
    );
}

#[test]
fn resolve_json_pointer_path() {
    let source = "server:\n  port: 8080\n";
    let map = SourceMap::new(source);
    assert_eq!(
        map.resolve_path("/server/port"),
        Some(SourceLocation::new(2, 3))
    );
}

#[test]
fn resolve_array_element_in_yaml() {
    let source = "items:\n  - first\n  - second\n  - third\n";
    let map = SourceMap::new(source);
    let loc = map.resolve_path("items[1]");
    assert!(loc.is_some());
    let loc = loc.unwrap();
    assert_eq!(loc.line, 3);
}

#[test]
fn resolve_nonexistent_key_returns_none() {
    let source = "server:\n  port: 8080\n";
    let map = SourceMap::new(source);
    assert_eq!(map.resolve_path("nonexistent"), None);
}

#[test]
fn resolve_skips_comments_and_empty_lines() {
    let source = "# comment line\n\nserver:\n  # inline comment\n  port: 8080\n";
    let map = SourceMap::new(source);
    assert_eq!(
        map.resolve_path("server.port"),
        Some(SourceLocation::new(5, 3))
    );
}

#[test]
fn resolve_with_blank_lines_between_keys() {
    let source = "server:\n\n  port: 8080\n";
    let map = SourceMap::new(source);
    assert_eq!(
        map.resolve_path("server.port"),
        Some(SourceLocation::new(3, 3))
    );
}

#[test]
fn at_byte_offset_basic() {
    let source = "line one\nline two\n";
    let map = SourceMap::new(source);
    assert_eq!(map.at_byte_offset(0), SourceLocation::new(1, 1));
    assert_eq!(map.at_byte_offset(5), SourceLocation::new(1, 6));
    assert_eq!(map.at_byte_offset(9), SourceLocation::new(2, 1));
    assert_eq!(map.at_byte_offset(13), SourceLocation::new(2, 5));
}

#[test]
fn at_byte_offset_clamps_to_end() {
    let source = "hi\n";
    let map = SourceMap::new(source);
    let loc = map.at_byte_offset(100);
    assert_eq!(loc.line, 2);
}

#[test]
fn resolve_single_quoted_key() {
    let source = "'server':\n  'port': 8080\n";
    let map = SourceMap::new(source);
    assert_eq!(
        map.resolve_path("server.port"),
        Some(SourceLocation::new(2, 3))
    );
}

#[test]
fn resolve_stops_at_dedent() {
    let source = "server:\n  port: 8080\nother:\n  port: 9090\n";
    let map = SourceMap::new(source);
    assert_eq!(
        map.resolve_path("server.port"),
        Some(SourceLocation::new(2, 3))
    );
    assert_eq!(
        map.resolve_path("other.port"),
        Some(SourceLocation::new(4, 3))
    );
}
