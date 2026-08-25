//! Shared data-generation utilities for handl benchmarks.
//!
//! Provides synthetic handl document generators at varying scales so that
//! each benchmark target can construct realistic inputs without
//! duplicating boilerplate.

use serde_json::{Value as JsonValue, json};

// ---------------------------------------------------------------------------
// YAML document generators
// ---------------------------------------------------------------------------

/// Generate a flat YAML document with `n` simple typed fields.
pub fn flat_yaml(n: usize) -> String {
    let mut lines = String::from("fields:\n");
    for i in 0..n {
        let ty = match i % 4 {
            0 => "string",
            1 => "number",
            2 => "boolean",
            _ => "object",
        };
        lines.push_str(&format!("  field_{i}:\n    type: {ty}\n"));
    }
    lines.push('\n');
    for i in 0..n {
        let val: &str = match i % 4 {
            0 => "\"value\"",
            1 => "42",
            2 => "true",
            _ => "{}",
        };
        lines.push_str(&format!("field_{i}: {val}\n"));
    }
    lines
}

/// Generate a YAML document where every field has an `expr:` declaration.
pub fn expr_yaml(n: usize) -> String {
    let mut lines = String::from("fields:\n");
    for i in 0..n {
        lines.push_str(&format!(
            "  field_{i}:\n    type: string\n    expr: \"hello_{i}\"\n"
        ));
    }
    lines.push('\n');
    for i in 0..n {
        lines.push_str(&format!("field_{i}: \"hello_{i}\"\n"));
    }
    lines
}

/// Generate a YAML document with `n` inheritable fields and `depth`
/// levels of nested overrides.
pub fn inherit_yaml(n: usize, depth: usize) -> String {
    let mut lines = String::from("fields:\n");
    for i in 0..n {
        lines.push_str(&format!(
            "  inh_{i}:\n    type: string\n    inheritable: true\n"
        ));
    }
    lines.push('\n');
    for i in 0..n {
        lines.push_str(&format!("inh_{i}: \"root_{i}\"\n"));
    }
    // Build nested data
    let mut current = String::from("\n");
    for d in 0..depth {
        let indent = "  ".repeat(d);
        current.push_str(&format!("{indent}level_{d}:\n"));
        let inner = "  ".repeat(d + 1);
        for i in 0..n {
            current.push_str(&format!("{inner}inh_{i}: \"depth_{d}_{i}\"\n"));
        }
    }
    lines.push_str(&current);
    lines
}

/// Generate a YAML document with `n` immutable fields and `depth`
/// levels of nested data (some overrides, some not).
pub fn mutable_yaml(n: usize, depth: usize) -> String {
    let mut lines = String::from("fields:\n");
    for i in 0..n {
        lines.push_str(&format!(
            "  imm_{i}:\n    type: string\n    mutable: false\n"
        ));
    }
    lines.push('\n');
    for i in 0..n {
        lines.push_str(&format!("imm_{i}: \"root_{i}\"\n"));
    }
    // Nested data with same values (no violations, just traversal cost)
    let mut current = String::from("\n");
    for d in 0..depth {
        let indent = "  ".repeat(d);
        current.push_str(&format!("{indent}level_{d}:\n"));
        let inner = "  ".repeat(d + 1);
        for i in 0..n {
            current.push_str(&format!("{inner}imm_{i}: \"root_{i}\"\n"));
        }
    }
    lines.push_str(&current);
    lines
}

/// Generate a YAML document with `n` merge fields.
pub fn merge_yaml(n: usize) -> String {
    let mut lines = String::from("fields:\n");
    for i in 0..n {
        lines.push_str(&format!(
            "  merge_{i}:\n    type: object\n    merge: true\n    properties:\n      prop_{i}: {{ type: string }}\n"
        ));
    }
    lines.push('\n');
    for i in 0..n {
        lines.push_str(&format!("merge_{i}:\n  prop_{i}: \"val_{i}\"\n"));
    }
    lines
}

/// Generate a mixed-feature YAML document with `n` total fields,
/// exercising expr, arg, inheritable, mutable, and merge.
pub fn mixed_yaml(n: usize) -> String {
    let mut lines = String::from("fields:\n");
    for i in 0..n {
        match i % 5 {
            0 => lines.push_str(&format!(
                "  field_{i}:\n    type: string\n    expr: \"hello_{i}\"\n"
            )),
            1 => lines.push_str(&format!(
                "  field_{i}:\n    type: number\n    arg:\n      type: number\n"
            )),
            2 => lines.push_str(&format!(
                "  field_{i}:\n    type: string\n    inheritable: true\n"
            )),
            3 => lines.push_str(&format!(
                "  field_{i}:\n    type: string\n    mutable: false\n"
            )),
            _ => lines.push_str(&format!(
                "  field_{i}:\n    type: object\n    merge: true\n    properties:\n      sub_{i}: {{ type: string }}\n"
            ))}
    }
    lines.push('\n');
    for i in 0..n {
        match i % 5 {
            0 => lines.push_str(&format!("field_{i}: \"hello_{i}\"\n")),
            1 => lines.push_str(&format!("field_{i}: {i}\n")),
            2 => lines.push_str(&format!("field_{i}: \"root_{i}\"\n")),
            3 => lines.push_str(&format!("field_{i}: \"root_{i}\"\n")),
            _ => lines.push_str(&format!("field_{i}:\n  sub_{i}: \"val_{i}\"\n")),
        }
    }
    lines
}

// ---------------------------------------------------------------------------
// Schema + data generators (for sub-crate benchmarks)
// ---------------------------------------------------------------------------

/// Generate `n` `(String, JsonValue)` schema pairs with simple types.
pub fn simple_schemas(n: usize) -> Vec<(String, JsonValue)> {
    (0..n)
        .map(|i| {
            let ty = match i % 4 {
                0 => "string",
                1 => "number",
                2 => "boolean",
                _ => "object",
            };
            (format!("field_{i}"), json!({ "type": ty }))
        })
        .collect()
}

/// Generate `n` schema pairs where every field has an `expr:` key.
pub fn expr_schemas(n: usize) -> Vec<(String, JsonValue)> {
    (0..n)
        .map(|i| {
            (
                format!("field_{i}"),
                json!({ "type": "string", "expr": format!("\"hello_{i}\"") }),
            )
        })
        .collect()
}

/// Generate `n` schema pairs where every field has an `arg:` key.
pub fn arg_schemas(n: usize) -> Vec<(String, JsonValue)> {
    (0..n)
        .map(|i| {
            (
                format!("field_{i}"),
                json!({ "type": "number", "arg": { "type": "number" } }),
            )
        })
        .collect()
}

/// Generate `n` schema pairs where every field has `merge: true`.
pub fn merge_schemas(n: usize) -> Vec<(String, JsonValue)> {
    (0..n)
        .map(|i| {
            (
                format!("merge_{i}"),
                json!({
                    "type": "object",
                    "merge": true,
                    "properties": {
                        format!("prop_{i}"): { "type": "string" }
                    }
                }),
            )
        })
        .collect()
}

/// Generate `n` inheritable schema pairs.
pub fn inheritable_schemas(n: usize) -> Vec<(String, JsonValue)> {
    (0..n)
        .map(|i| {
            (
                format!("inh_{i}"),
                json!({ "type": "string", "inheritable": true }),
            )
        })
        .collect()
}

/// Generate nested JSON data with `n` inheritable fields at each of `depth` levels.
pub fn nested_data(n: usize, depth: usize) -> JsonValue {
    let mut root = serde_json::Map::new();
    for i in 0..n {
        root.insert(format!("inh_{i}"), json!(format!("root_{i}")));
    }
    let mut current = json!(root);
    for d in 0..depth {
        let mut level = serde_json::Map::new();
        for i in 0..n {
            level.insert(format!("inh_{i}"), json!(format!("depth_{d}_{i}")));
        }
        level.insert(format!("level_{d}"), current);
        current = json!(level);
    }
    current
}

/// Generate `n` immutable field names.
pub fn immutable_fields(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("imm_{i}")).collect()
}

/// Generate nested JSON data with `n` immutable fields at each of `depth` levels
/// (same values as root — no violations, just traversal cost).
pub fn nested_data_uniform(n: usize, depth: usize) -> JsonValue {
    let mut root = serde_json::Map::new();
    for i in 0..n {
        root.insert(format!("imm_{i}"), json!(format!("root_{i}")));
    }
    let mut current = json!(root);
    for d in 0..depth {
        let mut level = serde_json::Map::new();
        for i in 0..n {
            level.insert(format!("imm_{i}"), json!(format!("root_{i}")));
        }
        level.insert(format!("level_{d}"), current);
        current = json!(level);
    }
    current
}

// ---------------------------------------------------------------------------
// Expression generators (for inference benchmarks)
// ---------------------------------------------------------------------------

/// Generate a literal expression of the given type index.
pub fn literal_expr(i: usize) -> String {
    match i % 4 {
        0 => format!("\"string_{i}\""),
        1 => format!("{i}"),
        2 => "true".to_string(),
        _ => "null".to_string(),
    }
}

/// Generate a path lookup expression of `depth` segments.
pub fn path_expr(depth: usize) -> String {
    (0..depth)
        .map(|i| format!("seg_{i}"))
        .collect::<Vec<_>>()
        .join(".")
}

/// Generate a nested function call expression of `depth` levels.
pub fn nested_call_expr(depth: usize) -> String {
    if depth == 0 {
        return "42".to_string();
    }
    let inner = nested_call_expr(depth - 1);
    format!("add({inner}, 1)")
}

/// Generate `n` distinct expression strings (mix of literals, paths, calls).
pub fn mixed_exprs(n: usize) -> Vec<String> {
    (0..n)
        .map(|i| match i % 3 {
            0 => literal_expr(i),
            1 => path_expr(i % 5 + 1),
            _ => nested_call_expr(i % 5),
        })
        .collect()
}

/// Generate `n` expressions that call generic functions (array.map, array.filter, etc.).
/// These exercise the TraitRegistry + unification hot path.
pub fn generic_call_exprs(n: usize) -> Vec<String> {
    (0..n)
        .map(|i| match i % 4 {
            0 => format!("array.map(fn_{i}, arr_{i})"),
            1 => format!("array.filter(fn_{i}, arr_{i})"),
            2 => format!("array.reduce(fn_{i}, arr_{i}, init_{i})"),
            _ => format!("array.flat_map(fn_{i}, arr_{i})"),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Type generators (for compatibility benchmarks)
// ---------------------------------------------------------------------------

use handl_typecheck_core::Type;

/// Generate a `Type` variant from an index.
pub fn simple_type(i: usize) -> Type {
    match i % 4 {
        0 => Type::String,
        1 => Type::Number,
        2 => Type::Boolean,
        _ => Type::Null,
    }
}

/// Generate an object type with `n` properties.
pub fn object_type(n: usize) -> Type {
    let mut props = std::collections::BTreeMap::new();
    for i in 0..n {
        props.insert(format!("prop_{i}"), simple_type(i));
    }
    Type::Object(props)
}

/// Generate a nested object type with `depth` levels.
pub fn nested_object_type(depth: usize) -> Type {
    if depth == 0 {
        return Type::String;
    }
    let mut props = std::collections::BTreeMap::new();
    props.insert("nested".to_string(), nested_object_type(depth - 1));
    Type::Object(props)
}

/// Generate an array type wrapping `inner`.
pub fn array_type(inner: Type) -> Type {
    Type::Array(Box::new(inner))
}

// ---------------------------------------------------------------------------
// Schema fragment generators (for interpretation benchmarks)
// ---------------------------------------------------------------------------

/// Generate a simple JSON schema fragment (one of the primitive types).
pub fn simple_schema(i: usize) -> JsonValue {
    match i % 4 {
        0 => json!({ "type": "string" }),
        1 => json!({ "type": "number" }),
        2 => json!({ "type": "boolean" }),
        _ => json!({ "type": "null" }),
    }
}

/// Generate a JSON schema for an object with `n` properties.
pub fn object_schema(n: usize) -> JsonValue {
    let mut props = serde_json::Map::new();
    for i in 0..n {
        props.insert(format!("prop_{i}"), simple_schema(i));
    }
    json!({ "type": "object", "properties": props })
}

/// Generate a JSON schema for a nested object with `depth` levels.
pub fn nested_object_schema(depth: usize) -> JsonValue {
    if depth == 0 {
        return json!({ "type": "string" });
    }
    let mut props = serde_json::Map::new();
    props.insert("nested".to_string(), nested_object_schema(depth - 1));
    json!({ "type": "object", "properties": props })
}

/// Generate a JSON schema for an array of the given item schema.
pub fn array_schema(item: JsonValue) -> JsonValue {
    json!({ "type": "array", "items": item })
}
