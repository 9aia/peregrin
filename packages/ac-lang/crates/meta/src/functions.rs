use ac_dialect::Dialect;
use ac_dialect_canonical::AC_DIALECT;
use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use serde_json::Value as JsonValue;

// ── reflection ──────────────────────────────────────────────

/// `typeof(value) → string` — returns the type name of a value.
///
/// Returns one of: `"string"`, `"number"`, `"boolean"`, `"null"`,
/// `"array"`, `"object"`, `"function"`.
#[derive(Clone)]
pub struct TypeOf;

impl Function for TypeOf {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let v = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        let type_name = match v.inner() {
            JsonValue::Null => "null",
            JsonValue::Bool(_) => "boolean",
            JsonValue::Number(_) => "number",
            JsonValue::String(_) => "string",
            JsonValue::Array(_) => "array",
            JsonValue::Object(obj) => {
                if obj.get("body").is_some_and(|b| b.is_string()) {
                    "function"
                } else {
                    "object"
                }
            }
        };
        Ok(NonFunctionValue::new(type_name))
    }
}

/// `keys(object) → array<string>` — returns the key names of an object.
#[derive(Clone)]
pub struct Keys;

impl Function for Keys {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let v = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        let obj = v.as_object().ok_or(Error::TypeMismatch {
            expected: "object",
            got: "non-object",
        })?;
        let keys: Vec<JsonValue> = obj.keys().map(|k| JsonValue::String(k.clone())).collect();
        Ok(NonFunctionValue::new(keys))
    }
}

/// `is_function(value) → boolean` — returns true if the value is a
/// function value (an object with a `body` string field).
#[derive(Clone)]
pub struct IsFunction;

impl Function for IsFunction {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let v = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        Ok(NonFunctionValue::new(v.is_function()))
    }
}

// ── dynamic evaluation ──────────────────────────────────────

/// `eval(expr_string) → value` — evaluates an expression string at
/// runtime using the current variable scope.
///
/// The expression string is parsed and evaluated with the current
/// context's variables bound as parameters. Requires an `Eval`
/// callback to be set on the context.
#[derive(Clone)]
pub struct Eval;

impl Function for Eval {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let expr_str = args
            .first()
            .and_then(|v| v.as_str())
            .ok_or(Error::TypeMismatch {
                expected: "string",
                got: "non-string",
            })?;
        let vars = ctx.collect_vars();
        ctx.eval_expr(expr_str, &vars)
    }
}

// ── AST manipulation ────────────────────────────────────────

/// `parse(expr_string) → ast` — parses an expression string into a
/// JSON AST representation.
///
/// The AST is a JSON object with a `kind` field:
/// - `{"kind": "literal", "value": ..., "type": "string|number|boolean|null"}`
/// - `{"kind": "path", "path": "doc.name"}`
/// - `{"kind": "call", "name": "add", "args": [...]}`
#[derive(Clone)]
pub struct Parse;

impl Function for Parse {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let expr_str = args
            .first()
            .and_then(|v| v.as_str())
            .ok_or(Error::TypeMismatch {
                expected: "string",
                got: "non-string",
            })?;
        let ast = parse_expr_to_json(expr_str, &AC_DIALECT).ok_or(Error::PrimitiveError(
            format!("parse(): failed to parse expression: {expr_str}"),
        ))?;
        Ok(NonFunctionValue::new(ast))
    }
}

/// `ast_to_string(ast) → string` — serializes a JSON AST back to an
/// expression string.
#[derive(Clone)]
pub struct AstToString;

impl Function for AstToString {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let ast = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        let s = ast_to_string_inner(ast.inner(), &AC_DIALECT).ok_or(Error::PrimitiveError(
            "ast_to_string(): invalid AST structure".to_string(),
        ))?;
        Ok(NonFunctionValue::new(s))
    }
}

/// `ast_name(ast) → string|null` — returns the function name from a
/// call AST node, or null for non-call nodes.
#[derive(Clone)]
pub struct AstName;

impl Function for AstName {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let ast = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        let obj = ast.as_object().ok_or(Error::TypeMismatch {
            expected: "object",
            got: "non-object",
        })?;
        let kind = obj.get("kind").and_then(|k| k.as_str());
        if kind != Some("call") {
            return Ok(NonFunctionValue::new(JsonValue::Null));
        }
        let name = obj
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or(Error::PrimitiveError(
                "ast_name(): call node missing 'name' field".to_string(),
            ))?;
        Ok(NonFunctionValue::new(name))
    }
}

/// `ast_args(ast) → array<ast>` — returns the argument list from a
/// call AST node, or an empty array for non-call nodes.
#[derive(Clone)]
pub struct AstArgs;

impl Function for AstArgs {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let ast = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        let obj = ast.as_object().ok_or(Error::TypeMismatch {
            expected: "object",
            got: "non-object",
        })?;
        let kind = obj.get("kind").and_then(|k| k.as_str());
        if kind != Some("call") {
            return Ok(NonFunctionValue::new(Vec::<JsonValue>::new()));
        }
        let args_arr = obj
            .get("args")
            .and_then(|a| a.as_array())
            .ok_or(Error::PrimitiveError(
                "ast_args(): call node missing 'args' field".to_string(),
            ))?;
        Ok(NonFunctionValue::new(args_arr.clone()))
    }
}

/// `ast_is_call(ast) → boolean` — returns true if the AST node is a
/// call node.
#[derive(Clone)]
pub struct AstIsCall;

impl Function for AstIsCall {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let ast = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        let is_call = ast
            .as_object()
            .is_some_and(|obj| obj.get("kind").is_some_and(|k| k.as_str() == Some("call")));
        Ok(NonFunctionValue::new(is_call))
    }
}

// ── internal parsing ────────────────────────────────────────

/// Parse an expression string into a JSON AST representation.
fn parse_expr_to_json(s: &str, dialect: &dyn Dialect) -> Option<JsonValue> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let prefix = dialect.syntax().interpolation_prefix();
    let suffix = dialect.syntax().interpolation_suffix();
    let s = if s.starts_with(prefix) && s.ends_with(suffix) && s.len() > prefix.len() + suffix.len()
    {
        s[prefix.len()..s.len() - suffix.len()].trim()
    } else {
        s
    };

    if let Some(paren_pos) = dialect.parser().find_call_open(s)
        && s.ends_with(dialect.syntax().call_close())
        && paren_pos < s.len() - 1
        && !s[..paren_pos].trim().is_empty()
    {
        let name = s[..paren_pos].trim();
        let inner = &s[paren_pos + 1..s.len() - 1];
        let args: Vec<JsonValue> = dialect
            .parser()
            .split_args(inner)
            .iter()
            .filter_map(|arg| parse_expr_to_json(arg.trim(), dialect))
            .collect();
        return Some(serde_json::json!({
            "kind": "call",
            "name": name,
            "args": args
        }));
    }

    if let Some(s_val) = dialect.parser().parse_string_literal(s) {
        return Some(serde_json::json!({
            "kind": "literal",
            "value": s_val,
            "type": "string"
        }));
    }

    if let Some(n) = dialect.parser().parse_number_literal(s) {
        let value = if n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0 {
            serde_json::json!(n as i64)
        } else {
            serde_json::json!(n)
        };
        return Some(serde_json::json!({
            "kind": "literal",
            "value": value,
            "type": "number"
        }));
    }

    if let Some(b) = dialect.parser().parse_bool_literal(s) {
        return Some(serde_json::json!({
            "kind": "literal",
            "value": b,
            "type": "boolean"
        }));
    }

    if dialect.parser().is_null_literal(s) {
        return Some(serde_json::json!({
            "kind": "literal",
            "value": null,
            "type": "null"
        }));
    }

    Some(serde_json::json!({
        "kind": "path",
        "path": s
    }))
}

/// Serialize a JSON AST back to an expression string.
fn ast_to_string_inner(ast: &JsonValue, dialect: &dyn Dialect) -> Option<String> {
    let obj = ast.as_object()?;
    let kind = obj.get("kind")?.as_str()?;

    match kind {
        "literal" => {
            let value = obj.get("value")?;
            let lit_type = obj.get("type").and_then(|t| t.as_str()).unwrap_or("");
            match lit_type {
                "string" => {
                    let s = value.as_str()?;
                    let quote = if s.contains('"') { '\'' } else { '"' };
                    Some(format!("{quote}{s}{quote}"))
                }
                "number" | "boolean" => Some(value.to_string()),
                "null" => Some("null".to_string()),
                _ => None,
            }
        }
        "path" => {
            let path = obj.get("path")?.as_str()?;
            Some(path.to_string())
        }
        "call" => {
            let name = obj.get("name")?.as_str()?;
            let args = obj.get("args")?.as_array()?;
            let arg_strs: Vec<String> = args
                .iter()
                .filter_map(|a| ast_to_string_inner(a, dialect))
                .collect();
            let open = dialect.syntax().call_open();
            let close = dialect.syntax().call_close();
            Some(format!("{name}{open}{}{close}", arg_strs.join(", ")))
        }
        _ => None,
    }
}
