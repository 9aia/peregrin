use ac_dialect::Dialect;
use ac_runtime::NonFunctionValue;
use ac_runtime_canonical::{CanonicalContext, Context, Error, Eval, Host, Vm};
use serde_json::Value as JsonValue;

use crate::parser::parse_expr_to_tree;
use crate::types::{DEFAULT_NAMESPACE, Namespaces};

/// Evaluates a single expression to a `JsonValue`.
///
/// The expression uses the syntax defined by the dialect:
/// - Function calls: `add(1, 2)`, `mul(x, 2)`
/// - Path lookups: `doc.name`, `env.HOME`, `name` (shorthand for `doc.name`)
/// - Literals: `"string"`, `123`, `true`, `false`, `null`
///
/// Returns `None` if the expression cannot be resolved.
pub fn eval_expression<H: Host + Clone + 'static>(
    expr: &str,
    namespaces: &Namespaces,
    host: &H,
    dialect: &'static dyn Dialect,
) -> Option<JsonValue> {
    let expr = expr.trim();
    if expr.is_empty() {
        return None;
    }

    let tree = parse_expr_to_tree(expr, dialect, namespaces)?;
    let mut ctx = CanonicalContext::from_namespaces(namespaces);
    let eval = VmEval::new(host.clone(), namespaces.clone(), dialect);
    ctx.set_eval(Box::new(eval.clone()));
    let vm = Vm::new(host.clone());
    vm.eval(&tree, &mut ctx).ok().map(|v| v.into_inner())
}

/// Evaluates a user-defined function body expression.
///
/// Params are merged into the default namespace (shadowing fields
/// with the same name), so bare names like `x` resolve via shorthand.
/// The host is passed through, so the body can call host primitives
/// and other user-defined functions.
///
/// Returns `None` if the body cannot be evaluated.
pub fn eval_body<H: Host + Clone + 'static>(
    body: &str,
    params: &[(String, JsonValue)],
    namespaces: &Namespaces,
    host: &H,
    dialect: &'static dyn Dialect,
) -> Option<JsonValue> {
    let mut merged = namespaces.clone();
    let mut doc_obj = merged
        .get(DEFAULT_NAMESPACE)
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    for (name, value) in params {
        doc_obj.insert(name.clone(), value.clone());
    }
    merged.insert(DEFAULT_NAMESPACE.to_string(), JsonValue::Object(doc_obj));
    eval_expression(body, &merged, host, dialect)
}

#[derive(Clone)]
struct VmEval<H: Host + Clone + 'static> {
    host: H,
    namespaces: Namespaces,
    dialect: &'static dyn Dialect,
}

impl<H: Host + Clone + 'static> VmEval<H> {
    fn new(host: H, namespaces: Namespaces, dialect: &'static dyn Dialect) -> Self {
        VmEval {
            host,
            namespaces,
            dialect,
        }
    }
}

impl<H: Host + Clone + 'static> Eval for VmEval<H> {
    fn eval_expr(
        &self,
        expr: &str,
        vars: &[(String, NonFunctionValue)],
    ) -> Result<NonFunctionValue, Error> {
        let mut doc = self
            .namespaces
            .get(DEFAULT_NAMESPACE)
            .cloned()
            .unwrap_or(JsonValue::Object(serde_json::Map::new()));
        if let Some(obj) = doc.as_object_mut() {
            for (name, value) in vars {
                obj.insert(name.clone(), value.inner().clone());
            }
        }

        let mut ctx = CanonicalContext::from_namespaces(&self.namespaces);
        ctx.define(DEFAULT_NAMESPACE, NonFunctionValue::new(doc));
        for (name, value) in vars {
            ctx.define(name.clone(), value.clone());
        }

        let tree = parse_expr_to_tree(expr, self.dialect, &self.namespaces)
            .ok_or_else(|| Error::PrimitiveError(format!("failed to parse expression: {expr}")))?;

        ctx.set_eval(Box::new(self.clone()));
        let vm = Vm::new(self.host.clone());
        vm.eval(&tree, &mut ctx)
    }
}
