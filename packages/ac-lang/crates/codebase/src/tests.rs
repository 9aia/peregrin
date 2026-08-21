use ac_runtime::FunctionValue;

use ac_store::DefinitionHash;

use crate::document::Document;
use crate::store::DocumentStore;

// ── Document tests ───────────────────────────────────────────────

fn make_func(body: &str) -> FunctionValue {
    FunctionValue {
        params: vec![],
        return_schema: None,
        body: body.to_string(),
    }
}

#[test]
fn document_hash_is_deterministic() {
    let doc1 = Document::new()
        .title("test")
        .entry("add", make_func("add(1, 2)"));
    let doc2 = Document::new()
        .title("test")
        .entry("add", make_func("add(1, 2)"));
    assert_eq!(doc1.hash(), doc2.hash());
}

#[test]
fn document_hash_differs_for_different_entries() {
    let doc1 = Document::new().entry("add", make_func("add(1, 2)"));
    let doc2 = Document::new().entry("add", make_func("add(1, 3)"));
    assert_ne!(doc1.hash(), doc2.hash());
}

#[test]
fn document_hash_is_entry_order_independent() {
    let doc1 = Document::new()
        .entry("a", make_func("1"))
        .entry("b", make_func("2"));
    let doc2 = Document::new()
        .entry("b", make_func("2"))
        .entry("a", make_func("1"));
    assert_eq!(doc1.hash(), doc2.hash());
}

#[test]
fn document_hash_includes_imports() {
    let import_hash = DefinitionHash::from_hex("a".repeat(64));
    let doc1 = Document::new().import(import_hash.clone());
    let doc2 = Document::new();
    assert_ne!(doc1.hash(), doc2.hash());
}

#[test]
fn document_hash_is_import_order_independent() {
    let h1 = DefinitionHash::from_hex("a".repeat(64));
    let h2 = DefinitionHash::from_hex("b".repeat(64));
    let doc1 = Document::new().import(h1.clone()).import(h2.clone());
    let doc2 = Document::new().import(h2.clone()).import(h1.clone());
    assert_eq!(doc1.hash(), doc2.hash());
}

#[test]
fn document_get_by_name() {
    let doc = Document::new()
        .entry("add", make_func("add(1, 2)"))
        .entry("mul", make_func("mul(3, 4)"));
    assert!(doc.get("add").is_some());
    assert!(doc.get("mul").is_some());
    assert!(doc.get("missing").is_none());
}

// ── DocumentStore tests ──────────────────────────────────────────

#[test]
fn store_insert_and_get() {
    let mut store = DocumentStore::new();
    let doc = Document::new().title("test").entry("f", make_func("42"));
    let hash = store.insert(doc);
    assert!(store.contains(&hash));
    let retrieved = store.get(&hash).unwrap();
    assert_eq!(retrieved.title, Some("test".to_string()));
}

#[test]
fn store_deduplicates_identical_documents() {
    let mut store = DocumentStore::new();
    let doc = Document::new().entry("f", make_func("42"));
    let h1 = store.insert(doc.clone());
    let h2 = store.insert(doc);
    assert_eq!(h1, h2);
    assert_eq!(store.len(), 1);
}

#[test]
fn store_resolve_imports() {
    let mut store = DocumentStore::new();

    // Base document (no imports)
    let base = Document::new().entry("base_func", make_func("1"));
    let base_hash = store.insert(base);

    // Dependent document (imports base)
    let dep = Document::new()
        .entry("dep_func", make_func("2"))
        .import(base_hash.clone());
    let dep_hash = store.insert(dep);

    let resolved = store.resolve_imports(&dep_hash);
    // Base should come first (dependency order), then dep
    assert_eq!(resolved.len(), 2);
    assert_eq!(resolved[0], base_hash);
    assert_eq!(resolved[1], dep_hash);
}

#[test]
fn store_resolve_imports_handles_self_import() {
    let mut store = DocumentStore::new();

    // A document that imports a non-existent hash (simulating a
    // self-reference that can't exist in content-addressing).
    // Should not infinite-loop.
    let fake = DefinitionHash::from_hex("a".repeat(64));
    let doc = Document::new()
        .entry("f", make_func("42"))
        .import(fake.clone());
    let hash = store.insert(doc);

    let resolved = store.resolve_imports(&hash);
    // Only the document itself (fake import is missing from store)
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0], hash);
}

#[test]
fn store_resolve_imports_missing_import() {
    let mut store = DocumentStore::new();
    let missing = DefinitionHash::from_hex("z".repeat(64));
    let doc = Document::new().entry("f", make_func("42")).import(missing);
    let hash = store.insert(doc);

    // Missing import is skipped, document itself is still resolved
    let resolved = store.resolve_imports(&hash);
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0], hash);
}

// ── edge case tests ───────────────────────────────────────────────

#[test]
fn empty_document_hash_is_deterministic() {
    let d1 = Document::new();
    let d2 = Document::new();
    assert_eq!(d1.hash(), d2.hash());
}

#[test]
fn document_with_title_only() {
    let doc = Document::new().title("empty");
    assert_eq!(doc.title, Some("empty".to_string()));
    assert!(doc.entries.is_empty());
    assert!(doc.imports.is_empty());
}

#[test]
fn store_empty_len_zero() {
    let store = DocumentStore::new();
    assert_eq!(store.len(), 0);
}

#[test]
fn store_get_missing_returns_none() {
    let store = DocumentStore::new();
    let hash = DefinitionHash::from_hex("a".repeat(64));
    assert!(store.get(&hash).is_none());
}

#[test]
fn store_contains_missing_returns_false() {
    let store = DocumentStore::new();
    let hash = DefinitionHash::from_hex("a".repeat(64));
    assert!(!store.contains(&hash));
}

#[test]
fn store_resolve_imports_chain() {
    let mut store = DocumentStore::new();

    // base → mid → top
    let base = Document::new().entry("base", make_func("1"));
    let base_hash = store.insert(base);

    let mid = Document::new()
        .entry("mid", make_func("2"))
        .import(base_hash.clone());
    let mid_hash = store.insert(mid);

    let top = Document::new()
        .entry("top", make_func("3"))
        .import(mid_hash.clone());
    let top_hash = store.insert(top);

    let resolved = store.resolve_imports(&top_hash);
    // base first, then mid, then top
    assert_eq!(resolved.len(), 3);
    assert_eq!(resolved[0], base_hash);
    assert_eq!(resolved[1], mid_hash);
    assert_eq!(resolved[2], top_hash);
}

#[test]
fn store_resolve_imports_diamond() {
    let mut store = DocumentStore::new();

    // base imported by both left and right, top imports both
    //    base
    //   /    \
    // left  right
    //   \    /
    //    top
    let base = Document::new().entry("base", make_func("1"));
    let base_hash = store.insert(base);

    let left = Document::new()
        .entry("left", make_func("2"))
        .import(base_hash.clone());
    let left_hash = store.insert(left);

    let right = Document::new()
        .entry("right", make_func("3"))
        .import(base_hash.clone());
    let right_hash = store.insert(right);

    let top = Document::new()
        .entry("top", make_func("4"))
        .import(left_hash.clone())
        .import(right_hash.clone());
    let top_hash = store.insert(top);

    let resolved = store.resolve_imports(&top_hash);
    // base should appear only once (dedup), then left and right, then top
    assert!(resolved.len() >= 4);
    // base must come before left and right
    let base_idx = resolved.iter().position(|h| h == &base_hash).unwrap();
    let left_idx = resolved.iter().position(|h| h == &left_hash).unwrap();
    let right_idx = resolved.iter().position(|h| h == &right_hash).unwrap();
    let top_idx = resolved.iter().position(|h| h == &top_hash).unwrap();
    assert!(base_idx < left_idx);
    assert!(base_idx < right_idx);
    assert!(left_idx < top_idx);
    assert!(right_idx < top_idx);
}

#[test]
fn document_hash_differs_with_and_without_title() {
    let d1 = Document::new().entry("f", make_func("42"));
    let d2 = Document::new().title("named").entry("f", make_func("42"));
    assert_ne!(d1.hash(), d2.hash());
}

#[test]
fn document_hash_differs_for_different_titles() {
    let d1 = Document::new().title("a").entry("f", make_func("42"));
    let d2 = Document::new().title("b").entry("f", make_func("42"));
    assert_ne!(d1.hash(), d2.hash());
}

#[test]
fn document_get_entry_by_name() {
    let doc = Document::new()
        .entry("f1", make_func("1"))
        .entry("f2", make_func("2"));
    let f1 = doc.get("f1").unwrap();
    assert_eq!(f1.function.body, "1");
    let f2 = doc.get("f2").unwrap();
    assert_eq!(f2.function.body, "2");
}

#[test]
fn store_multiple_documents() {
    let mut store = DocumentStore::new();
    let docs: Vec<Document> = (0..5)
        .map(|i| Document::new().entry("f", make_func(&format!("add({i}, 1)"))))
        .collect();

    let mut hashes = Vec::new();
    for d in docs {
        hashes.push(store.insert(d));
    }
    assert_eq!(store.len(), 5);
    for h in &hashes {
        assert!(store.contains(h));
    }
}
