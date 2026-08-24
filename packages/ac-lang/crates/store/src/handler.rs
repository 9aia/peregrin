use ac_runtime_values::NonFunctionValue;
use ac_ast_canonical::{Expr, HandlerClause, Pattern};
use serde_json::Value as JsonValue;

use crate::canonical::canonical_string;
use crate::hash::DefinitionHash;

type RuntimeExpr = Expr<NonFunctionValue>;
type RuntimeHandlerClause = HandlerClause<NonFunctionValue>;

/// Convert a handler clause to canonical JSON for hashing.
///
/// The JSON structure is:
/// ```json
/// {
///   "op": "fetch",
///   "params": ["url"],
///   "resume": "k",
///   "body": { ... expr json ... }
/// }
/// ```
pub fn handler_clause_to_json(clause: &RuntimeHandlerClause) -> JsonValue {
    JsonValue::Object(serde_json::Map::from_iter([
        ("op".to_string(), JsonValue::String(clause.op_name.clone())),
        (
            "params".to_string(),
            JsonValue::Array(
                clause
                    .params
                    .iter()
                    .map(|p| JsonValue::String(p.clone()))
                    .collect(),
            ),
        ),
        (
            "resume".to_string(),
            JsonValue::String(clause.continuation_name.clone()),
        ),
        ("body".to_string(), expr_to_json(&clause.body)),
    ]))
}

/// Convert a pattern to canonical JSON for hashing.
fn pattern_to_json(pattern: &Pattern) -> JsonValue {
    match pattern {
        Pattern::Identifier(name) => JsonValue::Object(serde_json::Map::from_iter([
            ("type".to_string(), JsonValue::String("identifier".to_string())),
            ("name".to_string(), JsonValue::String(name.clone())),
        ])),
        Pattern::Array(elements) => JsonValue::Object(serde_json::Map::from_iter([
            ("type".to_string(), JsonValue::String("array".to_string())),
            (
                "elements".to_string(),
                JsonValue::Array(elements.iter().map(pattern_to_json).collect()),
            ),
        ])),
        Pattern::Object(entries) => JsonValue::Object(serde_json::Map::from_iter([
            ("type".to_string(), JsonValue::String("object".to_string())),
            (
                "entries".to_string(),
                JsonValue::Array(
                    entries
                        .iter()
                        .map(|(k, v)| {
                            JsonValue::Array(vec![JsonValue::String(k.clone()), pattern_to_json(v)])
                        })
                        .collect(),
                ),
            ),
        ])),
        Pattern::Rest(name) => JsonValue::Object(serde_json::Map::from_iter([
            ("type".to_string(), JsonValue::String("rest".to_string())),
            ("name".to_string(), JsonValue::String(name.clone())),
        ])),
    }
}

/// Convert an expression to canonical JSON for hashing.
fn expr_to_json(expr: &RuntimeExpr) -> JsonValue {
    match expr {
        Expr::Value(value) => JsonValue::Object(serde_json::Map::from_iter([
            ("type".to_string(), JsonValue::String("value".to_string())),
            ("value".to_string(), value.inner().clone()),
        ])),
        Expr::Call { name, args, .. } => JsonValue::Object(serde_json::Map::from_iter([
            ("type".to_string(), JsonValue::String("call".to_string())),
            ("name".to_string(), JsonValue::String(name.clone())),
            (
                "args".to_string(),
                JsonValue::Array(args.iter().map(expr_to_json).collect()),
            ),
        ])),
        Expr::Apply { target, args } => JsonValue::Object(serde_json::Map::from_iter([
            ("type".to_string(), JsonValue::String("apply".to_string())),
            ("target".to_string(), expr_to_json(target)),
            (
                "args".to_string(),
                JsonValue::Array(args.iter().map(expr_to_json).collect()),
            ),
        ])),
        Expr::Handle { body, handlers } => JsonValue::Object(serde_json::Map::from_iter([
            ("type".to_string(), JsonValue::String("handle".to_string())),
            ("body".to_string(), expr_to_json(body)),
            (
                "handlers".to_string(),
                JsonValue::Array(handlers.iter().map(handler_clause_to_json).collect()),
            ),
        ])),
        Expr::Array(elements) => JsonValue::Object(serde_json::Map::from_iter([
            ("type".to_string(), JsonValue::String("array".to_string())),
            (
                "elements".to_string(),
                JsonValue::Array(elements.iter().map(expr_to_json).collect()),
            ),
        ])),
        Expr::Object(entries) => JsonValue::Object(serde_json::Map::from_iter([
            ("type".to_string(), JsonValue::String("object".to_string())),
            (
                "entries".to_string(),
                JsonValue::Array(
                    entries
                        .iter()
                        .map(|(k, v)| {
                            JsonValue::Array(vec![JsonValue::String(k.clone()), expr_to_json(v)])
                        })
                        .collect(),
                ),
            ),
        ])),
        Expr::Var(name) => JsonValue::Object(serde_json::Map::from_iter([
            ("type".to_string(), JsonValue::String("var".to_string())),
            ("name".to_string(), JsonValue::String(name.clone())),
        ])),
        Expr::Assign { target, value } => JsonValue::Object(serde_json::Map::from_iter([
            ("type".to_string(), JsonValue::String("assign".to_string())),
            ("target".to_string(), pattern_to_json(target)),
            ("value".to_string(), expr_to_json(value)),
        ])),
        Expr::Function {
            type_params,
            params,
            ret,
            body,
        } => JsonValue::Object(serde_json::Map::from_iter([
            ("type".to_string(), JsonValue::String("function".to_string())),
            (
                "type_params".to_string(),
                JsonValue::Array(type_params.iter().map(|s| JsonValue::String(s.clone())).collect()),
            ),
            (
                "params".to_string(),
                JsonValue::Array(
                    params
                        .iter()
                        .map(|(pat, t)| {
                            JsonValue::Array(vec![
                                pattern_to_json(pat),
                                t.as_deref().map(expr_to_json).unwrap_or(JsonValue::Null),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "ret".to_string(),
                ret.as_deref().map(expr_to_json).unwrap_or(JsonValue::Null),
            ),
            ("body".to_string(), JsonValue::String(body.clone())),
        ])),
    }
}

/// Compute the content hash of a handler clause.
pub fn hash_handler_clause(clause: &RuntimeHandlerClause) -> DefinitionHash {
    let json = handler_clause_to_json(clause);
    let canonical = canonical_string(&json);
    DefinitionHash::from_canonical_string(&canonical)
}

/// Compute the content hash of an expression.
pub fn hash_expr(expr: &RuntimeExpr) -> DefinitionHash {
    let json = expr_to_json(expr);
    let canonical = canonical_string(&json);
    DefinitionHash::from_canonical_string(&canonical)
}