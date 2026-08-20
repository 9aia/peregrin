use super::*;

#[test]
fn resolve_empty_path_returns_1_1() {
    let map = SourceMap::new();
    assert_eq!(map.resolve_path(""), Some(SourceLocation::new(1, 1)));
}

#[test]
fn insert_and_resolve() {
    let mut map = SourceMap::new();
    map.insert("server", SourceLocation::new(1, 1));
    map.insert("server.port", SourceLocation::new(2, 3));
    assert_eq!(
        map.resolve_path("server.port"),
        Some(SourceLocation::new(2, 3))
    );
}

#[test]
fn resolve_unknown_path_returns_none() {
    let map = SourceMap::new();
    assert_eq!(map.resolve_path("unknown"), None);
}

#[test]
fn empty_map_is_empty() {
    let map = SourceMap::new();
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);
}

#[test]
fn len_after_inserts() {
    let mut map = SourceMap::new();
    map.insert("a", SourceLocation::new(1, 1));
    map.insert("b", SourceLocation::new(2, 1));
    assert_eq!(map.len(), 2);
}

#[test]
fn default_is_empty() {
    let map = SourceMap::default();
    assert!(map.is_empty());
}

#[test]
fn source_location_display() {
    let loc = SourceLocation::new(3, 7);
    assert_eq!(loc.to_string(), "3:7");
}

#[test]
fn source_location_default_is_0_0() {
    let loc = SourceLocation::default();
    assert_eq!(loc, SourceLocation::new(0, 0));
}
