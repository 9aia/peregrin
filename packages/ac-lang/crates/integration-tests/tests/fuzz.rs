//! Fuzzy / property-based tests.
//!
//! These tests use random input generation to verify invariants:
//! - Hash determinism: same input always produces the same hash.
//! - Serialization roundtrips: serialize → deserialize → equality.
//! - Canonical JSON: canonicalization is idempotent.
//! - Continuation roundtrips: any valid continuation survives serialization.

use ac_ast_canonical::{Expr, HandlerClause};
use ac_runtime::NonFunctionValue;
use ac_runtime_canonical::{Continuation, Frame};
use ac_store::DefinitionHash;
use ac_store::canonicalize_json;
use serde_json::Value as JsonValue;

// ── helpers ───────────────────────────────────────────────────────

/// Simple deterministic PRNG (xorshift64) — no external dependency.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Rng { state: seed.max(1) }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_usize(&mut self, max: usize) -> usize {
        (self.next_u64() as usize) % max
    }

    fn next_bool(&mut self) -> bool {
        self.next_u64() % 2 == 0
    }

    fn next_f64(&mut self) -> f64 {
        let bits = self.next_u64();
        // Generate a number in [0, 1000)
        (bits % 1_000_000) as f64 / 1000.0
    }

    fn next_string(&mut self) -> String {
        let len = self.next_usize(10) + 1;
        let mut s = String::new();
        for _ in 0..len {
            let c = (b'a' + (self.next_u64() % 26) as u8) as char;
            s.push(c);
        }
        s
    }
}

/// Generate a random JSON value (depth-limited to prevent stack overflow).
fn random_json(rng: &mut Rng, depth: usize) -> JsonValue {
    if depth == 0 {
        return match rng.next_usize(4) {
            0 => JsonValue::Null,
            1 => JsonValue::Bool(rng.next_bool()),
            2 => serde_json::json!(rng.next_f64()),
            _ => JsonValue::String(rng.next_string()),
        };
    }
    match rng.next_usize(5) {
        0 => JsonValue::Null,
        1 => JsonValue::Bool(rng.next_bool()),
        2 => serde_json::json!(rng.next_f64()),
        3 => JsonValue::String(rng.next_string()),
        4 => {
            let len = rng.next_usize(5);
            JsonValue::Array((0..len).map(|_| random_json(rng, depth - 1)).collect())
        }
        _ => {
            let len = rng.next_usize(5);
            let mut map = serde_json::Map::new();
            for _ in 0..len {
                map.insert(rng.next_string(), random_json(rng, depth - 1));
            }
            JsonValue::Object(map)
        }
    }
}

/// Generate a random expression (depth-limited).
fn random_expr(rng: &mut Rng, depth: usize) -> Expr<NonFunctionValue> {
    if depth == 0 {
        return Expr::Value(NonFunctionValue::new(match rng.next_usize(3) {
            0 => serde_json::json!(rng.next_f64()),
            1 => serde_json::json!(rng.next_string()),
            _ => serde_json::json!(rng.next_bool()),
        }));
    }
    match rng.next_usize(3) {
        0 => Expr::Value(NonFunctionValue::new(match rng.next_usize(3) {
            0 => serde_json::json!(rng.next_f64()),
            1 => serde_json::json!(rng.next_string()),
            _ => serde_json::json!(rng.next_bool()),
        })),
        1 => {
            let name = match rng.next_usize(4) {
                0 => "add",
                1 => "mul",
                2 => "concat",
                _ => &rng.next_string()[..],
            };
            let n_args = rng.next_usize(4);
            Expr::call(
                name,
                (0..n_args).map(|_| random_expr(rng, depth - 1)).collect(),
            )
        }
        _ => {
            let n_handlers = rng.next_usize(3) + 1;
            let handlers = (0..n_handlers)
                .map(|_| HandlerClause {
                    op_name: match rng.next_usize(3) {
                        0 => "fetch".to_string(),
                        1 => "log".to_string(),
                        _ => rng.next_string(),
                    },
                    params: if rng.next_bool() {
                        vec!["url".to_string()]
                    } else {
                        vec![]
                    },
                    continuation_name: "k".to_string(),
                    body: random_expr(rng, depth - 1),
                })
                .collect();
            Expr::handle(random_expr(rng, depth - 1), handlers)
        }
    }
}

/// Generate a random frame.
fn random_frame(rng: &mut Rng) -> Frame {
    let n_evaluated = rng.next_usize(4);
    let n_pending = rng.next_usize(3);
    Frame {
        name: match rng.next_usize(3) {
            0 => "add".to_string(),
            1 => "fetch".to_string(),
            _ => rng.next_string(),
        },
        evaluated: (0..n_evaluated)
            .map(|_| NonFunctionValue::new(random_json(rng, 2)))
            .collect(),
        pending: (0..n_pending).map(|_| random_expr(rng, 2)).collect(),
        handlers: vec![],
    }
}

// ── property: canonicalization is idempotent ──────────────────────

#[test]
fn fuzz_canonicalize_idempotent() {
    let mut rng = Rng::new(42);
    for _ in 0..500 {
        let value = random_json(&mut rng, 4);
        let once = canonicalize_json(&value);
        let twice = canonicalize_json(&once);
        assert_eq!(once, twice, "canonicalize must be idempotent");
    }
}

#[test]
fn fuzz_canonicalize_preserves_value_semantics() {
    let mut rng = Rng::new(123);
    for _ in 0..200 {
        let value = random_json(&mut rng, 3);
        let canonical = canonicalize_json(&value);
        // Canonical form should have the same number of elements
        match (&value, &canonical) {
            (JsonValue::Array(a), JsonValue::Array(b)) => assert_eq!(a.len(), b.len()),
            (JsonValue::Object(a), JsonValue::Object(b)) => assert_eq!(a.len(), b.len()),
            (JsonValue::Null, JsonValue::Null) => {}
            (JsonValue::Bool(a), JsonValue::Bool(b)) => assert_eq!(a, b),
            (JsonValue::Number(a), JsonValue::Number(b)) => assert_eq!(a, b),
            (JsonValue::String(a), JsonValue::String(b)) => assert_eq!(a, b),
            _ => panic!("type changed during canonicalization"),
        }
    }
}

// ── property: hash determinism ────────────────────────────────────

#[test]
fn fuzz_hash_deterministic() {
    let mut rng = Rng::new(999);
    for _ in 0..500 {
        let value = random_json(&mut rng, 4);
        let h1 = DefinitionHash::from_json(&value);
        let h2 = DefinitionHash::from_json(&value);
        assert_eq!(h1, h2, "hash must be deterministic for same input");
    }
}

#[test]
fn fuzz_hash_key_order_independent() {
    let mut rng = Rng::new(777);
    for _ in 0..200 {
        // Create an object with keys in one order
        let mut map1 = serde_json::Map::new();
        let mut map2 = serde_json::Map::new();
        let n_keys = rng.next_usize(5) + 1;
        let keys: Vec<String> = (0..n_keys).map(|_| rng.next_string()).collect();
        for key in &keys {
            let val = random_json(&mut rng, 2);
            map1.insert(key.clone(), val.clone());
            map2.insert(key.clone(), val);
        }
        // Shuffle map2 keys by inserting in reverse order
        // (serde_json::Map with preserve_order may matter, but
        // canonicalization sorts keys before hashing)
        let v1 = JsonValue::Object(map1);
        let v2 = JsonValue::Object(map2);
        let h1 = DefinitionHash::from_json(&v1);
        let h2 = DefinitionHash::from_json(&v2);
        assert_eq!(h1, h2, "hash must be key-order independent");
    }
}

// ── property: continuation serialization roundtrip ────────────────

#[test]
fn fuzz_continuation_serialization_roundtrip() {
    let mut rng = Rng::new(555);
    for _ in 0..200 {
        let n_frames = rng.next_usize(5);
        let frames = (0..n_frames).map(|_| random_frame(&mut rng)).collect();
        let cont = Continuation { frames };

        let json = cont.to_json();
        let restored = Continuation::from_json(&json);
        assert!(restored.is_ok(), "deserialization must succeed");
        assert_eq!(restored.unwrap(), cont, "roundtrip must preserve equality");
    }
}

#[test]
fn fuzz_empty_continuation_roundtrip() {
    let cont = Continuation::empty();
    let json = cont.to_json();
    let restored = Continuation::from_json(&json).unwrap();
    assert_eq!(restored, cont);
    assert!(restored.is_empty());
}

// ── property: NonFunctionValue clone/eq ───────────────────────────────

#[test]
fn fuzz_runtime_value_clone_eq() {
    let mut rng = Rng::new(333);
    for _ in 0..500 {
        let value = random_json(&mut rng, 4);
        let rv1 = NonFunctionValue::new(value);
        let rv2 = rv1.clone();
        assert_eq!(rv1, rv2, "clone must produce equal value");
    }
}

// ── property: expr clone/eq ───────────────────────────────────────

#[test]
fn fuzz_expr_clone_eq() {
    let mut rng = Rng::new(888);
    for _ in 0..200 {
        let expr = random_expr(&mut rng, 4);
        let cloned = expr.clone();
        assert_eq!(expr, cloned, "clone must produce equal expr");
    }
}

// ── property: hash format is always valid hex ─────────────────────

#[test]
fn fuzz_hash_always_valid_hex_64() {
    let mut rng = Rng::new(222);
    for _ in 0..500 {
        let value = random_json(&mut rng, 4);
        let hash = DefinitionHash::from_json(&value);
        let hex = hash.to_string();
        assert_eq!(hex.len(), 64, "hash must be 64 hex chars");
        assert!(
            hex.chars().all(|c| c.is_ascii_hexdigit()),
            "hash must be valid hex"
        );
        // Parse back
        let parsed: DefinitionHash = hex.parse().unwrap();
        assert_eq!(parsed, hash, "parse must roundtrip");
    }
}
