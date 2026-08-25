//! JS backend for the handl language.
//!
//! Walks the canonical `Expr` tree and emits JavaScript source that
//! depends on the `@handl/vm-js` runtime.

use handl_ast_canonical::{Expr, HandlerClause, Pattern};
use handl_lexer_canonical::CanonicalLexer;
use handl_parser::Parser;
use handl_parser_canonical::CanonicalParser;
use handl_vm_values_json::NonFunctionValue;
use serde_json::Value as JsonValue;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("parse error: {0}")]
    Parse(String),
    #[error("unsupported expression: {0}")]
    Unsupported(String),
}

pub fn compile(expr: &Expr<NonFunctionValue>) -> Result<String, Error> {
    let mut out = String::new();
    compile_expr(expr, &mut out)?;
    Ok(out)
}

pub fn compile_module(expr: &Expr<NonFunctionValue>) -> Result<String, Error> {
    let body = compile(expr)?;
    Ok(format!(
        "import {{ Vm }} from '@handl/vm-js';\n\nexport default (host) => {{\n  const vm = new Vm(host);\n  const ctx = vm.context();\n  return vm.run(ctx, {body});\n}};\n"
    ))
}

fn compile_expr(expr: &Expr<NonFunctionValue>, out: &mut String) -> Result<(), Error> {
    out.push_str("(function* (ctx, vm) { ");
    compile_expr_body(expr, out)?;
    out.push_str(" })(ctx, vm)");
    Ok(())
}

fn compile_expr_body(expr: &Expr<NonFunctionValue>, out: &mut String) -> Result<(), Error> {
    match expr {
        Expr::Value(value) => {
            out.push_str("return ");
            out.push_str(&json_literal(value.inner()));
            out.push_str("; ");
        }
        Expr::Var(name) => {
            out.push_str("return vm.resolve(ctx, ");
            out.push_str(&js_string_literal(name));
            out.push_str("); ");
        }
        Expr::Assign { target, value } => {
            out.push_str("const $v = yield* ");
            compile_expr(value, out)?;
            out.push_str("; return vm.bindPattern(ctx, ");
            out.push_str(&pattern_literal(target));
            out.push_str(", $v); ");
        }
        Expr::Function {
            params, ret, body, ..
        } => {
            let body_expr = CanonicalParser
                .parse_text(body, &CanonicalLexer::default())
                .map_err(|e| Error::Parse(e.to_string()))?;
            out.push_str("return ({ body: ");
            out.push_str(&js_string_literal(body));
            out.push_str(", params: ");
            out.push_str(&params_literal(params)?);
            out.push_str(", \"return\": ");
            match ret {
                Some(type_expr) => compile_expr_body(type_expr, out)?,
                None => out.push_str("null"),
            }
            out.push_str(", __call: function* (ctx, vm, args) { ");
            for (i, (pat, _)) in params.iter().enumerate() {
                out.push_str("vm.bindPattern(ctx, ");
                out.push_str(&pattern_literal(pat));
                out.push_str(", args[");
                out.push_str(&i.to_string());
                out.push_str("]); ");
            }
            out.push_str("return yield* ");
            compile_expr(&body_expr, out)?;
            out.push_str("; } }); ");
        }
        Expr::Call { name, args, .. } => {
            out.push_str("const $args = [");
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str("yield* ");
                compile_expr(arg, out)?;
            }
            out.push_str("]; return yield* vm.callGen(ctx, ");
            out.push_str(&js_string_literal(name));
            out.push_str(", $args); ");
        }
        Expr::Apply { target, args } => {
            out.push_str("const $t = yield* ");
            compile_expr(target, out)?;
            out.push_str("; const $args = [");
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str("yield* ");
                compile_expr(arg, out)?;
            }
            out.push_str("]; return yield* vm.applyGen(ctx, $t, $args); ");
        }
        Expr::Handle { body, handlers } => {
            out.push_str("return vm.handle(ctx, (function* (ctx, vm) { return yield* ");
            compile_expr(body, out)?;
            out.push_str("; })(ctx, vm), [");
            for (i, clause) in handlers.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                compile_handler(clause, out)?;
            }
            out.push_str("]); ");
        }
        Expr::Array(elements) => {
            out.push_str("const $a = [");
            for (i, elem) in elements.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str("yield* ");
                compile_expr(elem, out)?;
            }
            out.push_str("]; return $a; ");
        }
        Expr::Object(entries) => {
            out.push_str("const $o = {}; ");
            for (key, value) in entries {
                out.push_str("$o[");
                out.push_str(&js_string_literal(key));
                out.push_str("] = yield* ");
                compile_expr(value, out)?;
                out.push_str("; ");
            }
            out.push_str("return $o; ");
        }
        Expr::Index { target, index } => {
            out.push_str("const $t = yield* ");
            compile_expr(target, out)?;
            out.push_str("; const $i = yield* ");
            compile_expr(index, out)?;
            out.push_str("; return vm.index($t, $i); ");
        }
        Expr::FieldAccess { target, field } => {
            out.push_str("const $t = yield* ");
            compile_expr(target, out)?;
            out.push_str("; return vm.field($t, ");
            out.push_str(&js_string_literal(field));
            out.push_str("); ");
        }
    }
    Ok(())
}

fn compile_handler(
    clause: &HandlerClause<NonFunctionValue>,
    out: &mut String,
) -> Result<(), Error> {
    out.push_str("{ op: ");
    out.push_str(&js_string_literal(&clause.op_name));
    out.push_str(", params: ");
    out.push_str(&json_literal(&JsonValue::Array(
        clause
            .params
            .iter()
            .map(|p| JsonValue::String(p.clone()))
            .collect(),
    )));
    out.push_str(", body: function* (ctx, vm, args, resume) { ");
    for name in &clause.params {
        out.push_str("vm.bindPattern(ctx, ");
        out.push_str(&pattern_literal(&Pattern::Identifier(name.clone())));
        out.push_str(", args[");
        out.push_str(&js_string_literal(name));
        out.push_str("]); ");
    }
    if !clause.continuation_name.is_empty() {
        out.push_str("vm.bindPattern(ctx, ");
        out.push_str(&pattern_literal(&Pattern::Identifier(
            clause.continuation_name.clone(),
        )));
        out.push_str(", resume); ");
    }
    out.push_str("return yield* ");
    compile_expr(&clause.body, out)?;
    out.push_str("; } }");
    Ok(())
}

fn params_literal(
    params: &[(Pattern, Option<Box<Expr<NonFunctionValue>>>)],
) -> Result<String, Error> {
    let mut items = Vec::new();
    for (pat, ty) in params {
        let schema = match ty {
            Some(type_expr) => compile(type_expr)?,
            None => "null".to_string(),
        };
        let name = pat.bound_names().first().cloned().unwrap_or_default();
        let pattern_json = pattern_literal(pat);
        items.push(format!(
            "{{ name: {}, schema: {}, pattern: {} }}",
            js_string_literal(&name),
            schema,
            pattern_json
        ));
    }
    Ok(format!("[{}]", items.join(", ")))
}

fn pattern_literal(pattern: &Pattern) -> String {
    serde_json::to_string(&pattern_json(pattern)).unwrap_or_default()
}

fn pattern_json(pattern: &Pattern) -> JsonValue {
    match pattern {
        Pattern::Identifier(name) => serde_json::json!({
            "type": "identifier",
            "name": name,
        }),
        Pattern::Array(elements) => serde_json::json!({
            "type": "array",
            "elements": elements.iter().map(pattern_json).collect::<Vec<_>>(),
        }),
        Pattern::Object(entries) => serde_json::json!({
            "type": "object",
            "entries": entries
                .iter()
                .map(|(key, pat)| serde_json::json!({ "key": key, "pattern": pattern_json(pat) }))
                .collect::<Vec<_>>(),
        }),
        Pattern::Rest(name) => serde_json::json!({
            "type": "rest",
            "name": name,
        }),
    }
}

fn json_literal(value: &JsonValue) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn js_string_literal(value: &str) -> String {
    json_literal(&JsonValue::String(value.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use handl_ast_canonical::Expr;
    use serde_json::json;

    #[test]
    fn compiles_a_call() {
        let expr = Expr::Call {
            name: "math.add".to_string(),
            args: vec![
                Expr::Value(NonFunctionValue::new(json!(1))),
                Expr::Value(NonFunctionValue::new(json!(2))),
            ],
            type_args: Vec::new(),
        };
        let js = compile(&expr).unwrap();
        assert!(js.contains("vm.callGen(ctx, \"math.add\", $args)"));
    }

    #[test]
    fn compiles_a_function() {
        let expr = Expr::Function {
            type_params: Vec::new(),
            params: vec![(Pattern::Identifier("x".to_string()), None)],
            ret: None,
            body: "add(x, 1)".to_string(),
        };
        let js = compile(&expr).unwrap();
        assert!(js.contains("__call"));
        assert!(js.contains("function*"));
    }

    #[test]
    fn compiles_a_module() {
        let expr = Expr::Call {
            name: "math.add".to_string(),
            args: vec![
                Expr::Value(NonFunctionValue::new(json!(1))),
                Expr::Value(NonFunctionValue::new(json!(2))),
            ],
            type_args: Vec::new(),
        };
        let js = compile_module(&expr).unwrap();
        assert!(js.contains("import { Vm } from '@handl/vm-js'"));
        assert!(js.contains("vm.run"));
    }
}
