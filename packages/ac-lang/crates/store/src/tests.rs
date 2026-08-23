use ac_runtime::FunctionValue;
use serde_json::json;

use ac_ast_canonical::{Expr, HandlerClause};
use ac_runtime::NonFunctionValue;

use crate::canonical::canonicalize_json;
use crate::eval_cache::{EvalCache, EvalKey};
use crate::handler::{hash_expr, hash_handler_clause};
use crate::hash::DefinitionHash;
use crate::store::{DefinitionStore, NameRegistry};

// ── DefinitionHash tests ─────────────────────────────────────────

#[test]
fn hash_is_deterministic() {
    let func = FunctionValue {
        params: vec![("x".to_string(), json!({"type": "number"}))],
        return_schema: None,
        body: "add(x, 1)".to_string(),
    };
    let h1 = DefinitionHash::from_json(&func.to_json());
    let h2 = DefinitionHash::from_json(&func.to_json());
    assert_eq!(h1, h2);
}

#[test]
fn hash_differs_for_different_bodies() {
    let f1 = FunctionValue {
        params: vec![],
        return_schema: None,
        body: "add(1, 2)".to_string(),
    };
    let f2 = FunctionValue {
        params: vec![],
        return_schema: None,
        body: "add(1, 3)".to_string(),
    };
    let h1 = DefinitionHash::from_json(&f1.to_json());
    let h2 = DefinitionHash::from_json(&f2.to_json());
    assert_ne!(h1, h2);
}

#[test]
fn hash_is_key_order_independent() {
    // Same content, different key order in the JSON
    let json1 = json!({"body": "add(1, 2)", "params": []});
    let json2 = json!({"params": [], "body": "add(1, 2)"});
    let h1 = DefinitionHash::from_json(&json1);
    let h2 = DefinitionHash::from_json(&json2);
    assert_eq!(h1, h2);
}

#[test]
fn hash_display_and_from_str() {
    let func = FunctionValue {
        params: vec![],
        return_schema: None,
        body: "42".to_string(),
    };
    let hash = DefinitionHash::from_json(&func.to_json());
    let hex_str = hash.to_string();
    assert_eq!(hex_str.len(), 64);

    let parsed: DefinitionHash = hex_str.parse().unwrap();
    assert_eq!(parsed, hash);
}

#[test]
fn hash_from_str_rejects_invalid() {
    assert!("short".parse::<DefinitionHash>().is_err());
    assert!("xyz".parse::<DefinitionHash>().is_err());
}

// ── canonicalize_json tests ──────────────────────────────────────

#[test]
fn canonicalize_sorts_keys() {
    let input = json!({"c": 1, "a": 2, "b": 3});
    let canonical = canonicalize_json(&input);
    let keys: Vec<&String> = canonical.as_object().unwrap().keys().collect();
    assert_eq!(keys, vec!["a", "b", "c"]);
}

#[test]
fn canonicalize_preserves_array_order() {
    let input = json!([3, 1, 2]);
    let canonical = canonicalize_json(&input);
    let arr = canonical.as_array().unwrap();
    assert_eq!(arr[0], 3);
    assert_eq!(arr[1], 1);
    assert_eq!(arr[2], 2);
}

#[test]
fn canonicalize_nested_objects() {
    let input = json!({"outer": {"z": 1, "a": 2}});
    let canonical = canonicalize_json(&input);
    let inner = canonical.get("outer").unwrap().as_object().unwrap();
    let keys: Vec<&String> = inner.keys().collect();
    assert_eq!(keys, vec!["a", "z"]);
}

// ── DefinitionStore tests ────────────────────────────────────────

#[test]
fn store_insert_and_get() {
    let mut store = DefinitionStore::new();
    let func = FunctionValue {
        params: vec![("x".to_string(), json!({"type": "number"}))],
        return_schema: None,
        body: "add(x, 1)".to_string(),
    };
    let hash = store.insert(func.clone());
    assert!(store.contains(&hash));
    let retrieved = store.get(&hash).unwrap();
    assert_eq!(retrieved.body, "add(x, 1)");
}

#[test]
fn store_deduplicates_identical_definitions() {
    let mut store = DefinitionStore::new();
    let func = FunctionValue {
        params: vec![],
        return_schema: None,
        body: "42".to_string(),
    };
    let h1 = store.insert(func.clone());
    let h2 = store.insert(func);
    assert_eq!(h1, h2);
    assert_eq!(store.len(), 1);
}

#[test]
fn store_serialization_roundtrip() {
    let mut store = DefinitionStore::new();
    let f1 = FunctionValue {
        params: vec![("x".to_string(), json!({"type": "number"}))],
        return_schema: None,
        body: "add(x, 1)".to_string(),
    };
    let f2 = FunctionValue {
        params: vec![],
        return_schema: Some(json!({"type": "number"})),
        body: "42".to_string(),
    };
    store.insert(f1);
    store.insert(f2);

    let json = store.to_json();
    let restored = DefinitionStore::from_json(&json);
    assert_eq!(restored.len(), store.len());
}

// ── NameRegistry tests ───────────────────────────────────────────

#[test]
fn name_registry_register_and_resolve() {
    let mut registry = NameRegistry::new();
    let hash = DefinitionHash::from_hex("a".repeat(64));
    registry.register("double", hash.clone());
    assert!(registry.contains("double"));
    let resolved = registry.resolve("double").unwrap();
    assert_eq!(resolved, &hash);
}

#[test]
fn name_registry_same_definition_multiple_names() {
    let mut registry = NameRegistry::new();
    let hash = DefinitionHash::from_hex("b".repeat(64));
    registry.register("double", hash.clone());
    registry.register("twice", hash.clone());
    assert_eq!(registry.resolve("double"), registry.resolve("twice"));
    assert_eq!(registry.len(), 2);
}

#[test]
fn name_registry_serialization_roundtrip() {
    let mut registry = NameRegistry::new();
    registry.register("add", DefinitionHash::from_hex("a".repeat(64)));
    registry.register("mul", DefinitionHash::from_hex("b".repeat(64)));

    let json = registry.to_json();
    let restored = NameRegistry::from_json(&json);
    assert_eq!(restored.len(), 2);
    assert!(restored.contains("add"));
    assert!(restored.contains("mul"));
}

// ── integration: store + registry ────────────────────────────────

#[test]
fn store_and_registry_integration() {
    let mut store = DefinitionStore::new();
    let mut registry = NameRegistry::new();

    let func = FunctionValue {
        params: vec![("x".to_string(), json!({"type": "number"}))],
        return_schema: None,
        body: "add(x, x)".to_string(),
    };

    let hash = store.insert(func);
    registry.register("double", hash.clone());

    // Resolve name → hash → definition
    let resolved_hash = registry.resolve("double").unwrap();
    let resolved_func = store.get(resolved_hash).unwrap();
    assert_eq!(resolved_func.body, "add(x, x)");
}

// ── handler clause hashing tests ─────────────────────────────────

#[test]
fn handler_clause_hash_is_deterministic() {
    let clause = HandlerClause {
        op_name: "fetch".to_string(),
        params: vec!["url".to_string()],
        continuation_name: "k".to_string(),
        body: Expr::call("k", vec![Expr::value(NonFunctionValue::new("cached"))]),
    };
    let h1 = hash_handler_clause(&clause);
    let h2 = hash_handler_clause(&clause);
    assert_eq!(h1, h2);
}

#[test]
fn handler_clause_hash_differs_for_different_ops() {
    let c1 = HandlerClause {
        op_name: "fetch".to_string(),
        params: vec![],
        continuation_name: "k".to_string(),
        body: Expr::value(NonFunctionValue::new(42)),
    };
    let c2 = HandlerClause {
        op_name: "read_file".to_string(),
        params: vec![],
        continuation_name: "k".to_string(),
        body: Expr::value(NonFunctionValue::new(42)),
    };
    assert_ne!(hash_handler_clause(&c1), hash_handler_clause(&c2));
}

#[test]
fn handler_clause_hash_differs_for_different_bodies() {
    let c1 = HandlerClause {
        op_name: "fetch".to_string(),
        params: vec![],
        continuation_name: "k".to_string(),
        body: Expr::value(NonFunctionValue::new(42)),
    };
    let c2 = HandlerClause {
        op_name: "fetch".to_string(),
        params: vec![],
        continuation_name: "k".to_string(),
        body: Expr::value(NonFunctionValue::new(99)),
    };
    assert_ne!(hash_handler_clause(&c1), hash_handler_clause(&c2));
}

// ── expr hashing tests ───────────────────────────────────────────

#[test]
fn expr_hash_value_is_deterministic() {
    let expr = Expr::value(NonFunctionValue::new(42));
    let h1 = hash_expr(&expr);
    let h2 = hash_expr(&expr);
    assert_eq!(h1, h2);
}

#[test]
fn expr_hash_call_is_deterministic() {
    let expr = Expr::call(
        "add",
        vec![
            Expr::value(NonFunctionValue::new(1)),
            Expr::value(NonFunctionValue::new(2)),
        ],
    );
    let h1 = hash_expr(&expr);
    let h2 = hash_expr(&expr);
    assert_eq!(h1, h2);
}

#[test]
fn expr_hash_handle_is_deterministic() {
    let expr = Expr::handle(
        Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec!["url".to_string()],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new("result")),
        }],
    );
    let h1 = hash_expr(&expr);
    let h2 = hash_expr(&expr);
    assert_eq!(h1, h2);
}

#[test]
fn expr_hash_differs_for_different_values() {
    let e1 = Expr::value(NonFunctionValue::new(42));
    let e2 = Expr::value(NonFunctionValue::new(99));
    assert_ne!(hash_expr(&e1), hash_expr(&e2));
}

// ── EvalCache tests ──────────────────────────────────────────────

#[test]
fn eval_cache_insert_and_get() {
    let mut cache = EvalCache::new();
    let hash = DefinitionHash::from_hex("a".repeat(64));
    let args = vec![NonFunctionValue::new(1), NonFunctionValue::new(2)];
    cache.insert(&hash, &args, NonFunctionValue::new(3));

    let result = cache.get(&hash, &args);
    assert_eq!(result.unwrap().as_number(), Some(3.0));
}

#[test]
fn eval_cache_miss_for_different_args() {
    let mut cache = EvalCache::new();
    let hash = DefinitionHash::from_hex("a".repeat(64));
    cache.insert(&hash, &[NonFunctionValue::new(1)], NonFunctionValue::new(42));

    assert!(cache.get(&hash, &[NonFunctionValue::new(2)]).is_none());
}

#[test]
fn eval_cache_miss_for_different_hash() {
    let mut cache = EvalCache::new();
    let h1 = DefinitionHash::from_hex("a".repeat(64));
    let h2 = DefinitionHash::from_hex("b".repeat(64));
    cache.insert(&h1, &[NonFunctionValue::new(1)], NonFunctionValue::new(42));

    assert!(cache.get(&h2, &[NonFunctionValue::new(1)]).is_none());
}

#[test]
fn eval_cache_serialization_roundtrip() {
    let mut cache = EvalCache::new();
    let h1 = DefinitionHash::from_hex("a".repeat(64));
    let h2 = DefinitionHash::from_hex("b".repeat(64));
    cache.insert(&h1, &[NonFunctionValue::new(1)], NonFunctionValue::new(42));
    cache.insert(&h2, &[NonFunctionValue::new("x")], NonFunctionValue::new("result"));

    let json = cache.to_json();
    let restored = EvalCache::from_json(&json);
    assert_eq!(restored.len(), 2);

    let result = restored.get(&h1, &[NonFunctionValue::new(1)]);
    assert_eq!(result.unwrap().as_number(), Some(42.0));
}

#[test]
fn eval_cache_key_deterministic() {
    let hash = DefinitionHash::from_hex("a".repeat(64));
    let args = vec![NonFunctionValue::new(1), NonFunctionValue::new("hello")];
    let k1 = EvalKey::new(&hash, &args);
    let k2 = EvalKey::new(&hash, &args);
    assert_eq!(k1, k2);
}

#[test]
fn eval_cache_empty_serialization() {
    let cache = EvalCache::new();
    let json = cache.to_json();
    let restored = EvalCache::from_json(&json);
    assert!(restored.is_empty());
}

// ── edge case tests ───────────────────────────────────────────────

#[test]
fn store_empty_to_json() {
    let store = DefinitionStore::new();
    let json = store.to_json();
    let restored = DefinitionStore::from_json(&json);
    assert!(restored.is_empty());
}

#[test]
fn store_get_missing_returns_none() {
    let store = DefinitionStore::new();
    let hash = DefinitionHash::from_hex("a".repeat(64));
    assert!(store.get(&hash).is_none());
}

#[test]
fn store_contains_missing_returns_false() {
    let store = DefinitionStore::new();
    let hash = DefinitionHash::from_hex("a".repeat(64));
    assert!(!store.contains(&hash));
}

#[test]
fn store_multiple_different_definitions() {
    let mut store = DefinitionStore::new();
    let funcs: Vec<FunctionValue> = (0..5)
        .map(|i| FunctionValue {
            params: vec![],
            return_schema: None,
            body: format!("add({i}, 1)"),
        })
        .collect();

    let mut hashes = Vec::new();
    for f in &funcs {
        hashes.push(store.insert(f.clone()));
    }

    assert_eq!(store.len(), 5);
    for (i, h) in hashes.iter().enumerate() {
        let retrieved = store.get(h).unwrap();
        assert_eq!(retrieved.body, format!("add({i}, 1)"));
    }
}

#[test]
fn name_registry_overwrite_name() {
    let mut registry = NameRegistry::new();
    let h1 = DefinitionHash::from_hex("a".repeat(64));
    let h2 = DefinitionHash::from_hex("b".repeat(64));

    registry.register("foo", h1.clone());
    registry.register("foo", h2.clone());

    assert_eq!(registry.resolve("foo").unwrap(), &h2);
    assert_eq!(registry.len(), 1);
}

#[test]
fn name_registry_resolve_missing_returns_none() {
    let registry = NameRegistry::new();
    assert!(registry.resolve("nonexistent").is_none());
}

#[test]
fn name_registry_empty_serialization() {
    let registry = NameRegistry::new();
    let json = registry.to_json();
    let restored = NameRegistry::from_json(&json);
    assert!(restored.is_empty());
}

#[test]
fn canonicalize_empty_object() {
    let input = json!({});
    let canonical = canonicalize_json(&input);
    assert!(canonical.as_object().unwrap().is_empty());
}

#[test]
fn canonicalize_empty_array() {
    let input = json!([]);
    let canonical = canonicalize_json(&input);
    assert!(canonical.as_array().unwrap().is_empty());
}

#[test]
fn canonicalize_primitive_passthrough() {
    assert_eq!(canonicalize_json(&json!(42)), json!(42));
    assert_eq!(canonicalize_json(&json!("hello")), json!("hello"));
    assert_eq!(canonicalize_json(&json!(true)), json!(true));
    assert_eq!(canonicalize_json(&json!(null)), json!(null));
}

#[test]
fn canonicalize_array_of_objects() {
    let input = json!([{"b": 2, "a": 1}, {"d": 4, "c": 3}]);
    let canonical = canonicalize_json(&input);
    let arr = canonical.as_array().unwrap();
    let keys0: Vec<&String> = arr[0].as_object().unwrap().keys().collect();
    let keys1: Vec<&String> = arr[1].as_object().unwrap().keys().collect();
    assert_eq!(keys0, vec!["a", "b"]);
    assert_eq!(keys1, vec!["c", "d"]);
}

#[test]
fn hash_differs_for_different_param_names() {
    let f1 = FunctionValue {
        params: vec![("x".to_string(), json!({"type": "number"}))],
        return_schema: None,
        body: "add(x, 1)".to_string(),
    };
    let f2 = FunctionValue {
        params: vec![("y".to_string(), json!({"type": "number"}))],
        return_schema: None,
        body: "add(y, 1)".to_string(),
    };
    let h1 = DefinitionHash::from_json(&f1.to_json());
    let h2 = DefinitionHash::from_json(&f2.to_json());
    assert_ne!(h1, h2);
}

#[test]
fn hash_differs_for_different_return_schema() {
    let f1 = FunctionValue {
        params: vec![],
        return_schema: Some(json!({"type": "number"})),
        body: "42".to_string(),
    };
    let f2 = FunctionValue {
        params: vec![],
        return_schema: Some(json!({"type": "string"})),
        body: "42".to_string(),
    };
    let h1 = DefinitionHash::from_json(&f1.to_json());
    let h2 = DefinitionHash::from_json(&f2.to_json());
    assert_ne!(h1, h2);
}

#[test]
fn hash_differs_with_and_without_return_schema() {
    let f1 = FunctionValue {
        params: vec![],
        return_schema: Some(json!({"type": "number"})),
        body: "42".to_string(),
    };
    let f2 = FunctionValue {
        params: vec![],
        return_schema: None,
        body: "42".to_string(),
    };
    let h1 = DefinitionHash::from_json(&f1.to_json());
    let h2 = DefinitionHash::from_json(&f2.to_json());
    assert_ne!(h1, h2);
}

#[test]
fn eval_cache_with_empty_args() {
    let mut cache = EvalCache::new();
    let hash = DefinitionHash::from_hex("a".repeat(64));
    cache.insert(&hash, &[], NonFunctionValue::new(42));
    let result = cache.get(&hash, &[]);
    assert_eq!(result.unwrap().as_number(), Some(42.0));
}

#[test]
fn eval_cache_overwrite_same_key() {
    let mut cache = EvalCache::new();
    let hash = DefinitionHash::from_hex("a".repeat(64));
    let args = vec![NonFunctionValue::new(1)];
    cache.insert(&hash, &args, NonFunctionValue::new(42));
    cache.insert(&hash, &args, NonFunctionValue::new(99));
    assert_eq!(cache.get(&hash, &args).unwrap().as_number(), Some(99.0));
    assert_eq!(cache.len(), 1);
}

#[test]
fn eval_cache_key_differs_for_different_arg_order() {
    let hash = DefinitionHash::from_hex("a".repeat(64));
    let k1 = EvalKey::new(&hash, &[NonFunctionValue::new(1), NonFunctionValue::new(2)]);
    let k2 = EvalKey::new(&hash, &[NonFunctionValue::new(2), NonFunctionValue::new(1)]);
    assert_ne!(k1, k2);
}

#[test]
fn handler_clause_hash_differs_for_different_params() {
    let c1 = HandlerClause {
        op_name: "fetch".to_string(),
        params: vec!["url".to_string()],
        continuation_name: "k".to_string(),
        body: Expr::value(NonFunctionValue::new(42)),
    };
    let c2 = HandlerClause {
        op_name: "fetch".to_string(),
        params: vec!["path".to_string()],
        continuation_name: "k".to_string(),
        body: Expr::value(NonFunctionValue::new(42)),
    };
    assert_ne!(hash_handler_clause(&c1), hash_handler_clause(&c2));
}

#[test]
fn handler_clause_hash_differs_for_different_continuation_name() {
    let c1 = HandlerClause {
        op_name: "fetch".to_string(),
        params: vec![],
        continuation_name: "k".to_string(),
        body: Expr::value(NonFunctionValue::new(42)),
    };
    let c2 = HandlerClause {
        op_name: "fetch".to_string(),
        params: vec![],
        continuation_name: "resume".to_string(),
        body: Expr::value(NonFunctionValue::new(42)),
    };
    assert_ne!(hash_handler_clause(&c1), hash_handler_clause(&c2));
}

#[test]
fn expr_hash_differs_for_different_call_names() {
    let e1 = Expr::call("add", vec![Expr::value(NonFunctionValue::new(1))]);
    let e2 = Expr::call("mul", vec![Expr::value(NonFunctionValue::new(1))]);
    assert_ne!(hash_expr(&e1), hash_expr(&e2));
}

#[test]
fn expr_hash_differs_for_different_arg_count() {
    let e1 = Expr::call("add", vec![Expr::value(NonFunctionValue::new(1))]);
    let e2 = Expr::call(
        "add",
        vec![
            Expr::value(NonFunctionValue::new(1)),
            Expr::value(NonFunctionValue::new(2)),
        ],
    );
    assert_ne!(hash_expr(&e1), hash_expr(&e2));
}

#[test]
fn expr_hash_handle_differs_for_different_handlers() {
    let e1 = Expr::handle(
        Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec![],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new("a")),
        }],
    );
    let e2 = Expr::handle(
        Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec![],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new("b")),
        }],
    );
    assert_ne!(hash_expr(&e1), hash_expr(&e2));
}
