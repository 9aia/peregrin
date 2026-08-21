//! Content addressing: hash-based definition storage and retrieval.
//!
//! This example demonstrates the content-addressed storage system:
//! definitions are identified by their SHA-256 hash, names are
//! resolved through a registry, and evaluation results can be cached.
//!
//! Run: `cargo run --example content-addressing`

use ac_runtime::FunctionValue;
use ac_store::{DefinitionStore, EvalCache, NameRegistry};

fn main() {
    println!("=== Content Addressing Demo ===");

    // ── Create a definition store and name registry ──────────────
    let mut store = DefinitionStore::new();
    let mut registry = NameRegistry::new();

    // Define some functions
    let double = FunctionValue {
        params: vec![("x".to_string(), serde_json::json!({"type": "number"}))],
        return_schema: Some(serde_json::json!({"type": "number"})),
        body: "mul(x, 2)".to_string(),
    };

    let increment = FunctionValue {
        params: vec![("x".to_string(), serde_json::json!({"type": "number"}))],
        return_schema: Some(serde_json::json!({"type": "number"})),
        body: "add(x, 1)".to_string(),
    };

    // Store them — each gets a content-addressed hash
    let double_hash = store.insert(double.clone());
    let increment_hash = store.insert(increment.clone());

    println!(
        "\n[Store] Inserted 'double' → hash: {}",
        &double_hash.to_string()[..16]
    );
    println!(
        "[Store] Inserted 'increment' → hash: {}",
        &increment_hash.to_string()[..16]
    );

    // Register human-readable names
    registry.register("double", double_hash.clone());
    registry.register("increment", increment_hash.clone());

    // ── Resolve by name → hash → definition ──────────────────────
    println!("\n[Registry] Resolving names:");

    let hash = registry.resolve("double").unwrap();
    let func = store.get(hash).unwrap();
    println!(
        "  double → hash {} → body: \"{}\"",
        &hash.to_string()[..16],
        func.body
    );

    let hash = registry.resolve("increment").unwrap();
    let func = store.get(hash).unwrap();
    println!(
        "  increment → hash {} → body: \"{}\"",
        &hash.to_string()[..16],
        func.body
    );

    // ── Verify determinism ───────────────────────────────────────
    println!("\n[Determinism] Re-inserting 'double':");
    let double_hash_2 = store.insert(double.clone());
    assert_eq!(double_hash, double_hash_2);
    println!("  Same hash: {} ✓", double_hash == double_hash_2);

    // ── Eval cache ───────────────────────────────────────────────
    println!("\n[EvalCache] Caching evaluation results:");
    let mut eval_cache = EvalCache::new();
    let args = vec![ac_runtime::NonFunctionValue::new(21)];
    eval_cache.insert(&double_hash, &args, ac_runtime::NonFunctionValue::new(42.0));

    let cached = eval_cache.get(&double_hash, &args).unwrap();
    println!("  double(21) → {} (cached)", cached.as_number().unwrap());

    // Serialize and restore the cache
    let cache_json = eval_cache.to_json();
    let restored_cache = EvalCache::from_json(&cache_json);
    assert_eq!(restored_cache.len(), 1);
    println!(
        "  Cache serialized and restored: {} entries ✓",
        restored_cache.len()
    );

    // ── Key properties ───────────────────────────────────────────
    println!("\n[Properties]");
    println!(
        "  Hash is 64 hex chars: {} ✓",
        double_hash.to_string().len() == 64
    );
    println!("  Store has {} definitions ✓", store.len());
    println!("  Registry has {} names ✓", registry.len());

    // Different definitions get different hashes
    assert_ne!(double_hash, increment_hash);
    println!("  Different definitions → different hashes ✓");

    println!("\nContent addressing demo passed.");
}
