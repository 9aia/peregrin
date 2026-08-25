use handl_vm::{Context, Error, Expr, Host};
use handl_vm_common::{ExprHandler, LegoVm};
use handl_vm_values_json::{FunctionValue, NonFunctionValue};
use serde_json::Value as JsonValue;

use crate::pattern::{define_pattern, pattern_to_json};
use crate::vm::{json_kind, qualified_name};

type RuntimeExpr = Expr<NonFunctionValue>;

/// Handles literal value expressions.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValueHandler;

impl ExprHandler for ValueHandler {
    fn can_handle(&self, expr: &RuntimeExpr) -> bool {
        matches!(expr, Expr::Value(_))
    }

    fn eval(
        &self,
        expr: &RuntimeExpr,
        _ctx: &mut dyn Context<NonFunctionValue>,
        _host: &dyn Host<NonFunctionValue>,
        _dispatch: &mut dyn FnMut(
            &RuntimeExpr,
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        match expr {
            Expr::Value(value) => Ok(value.clone()),
            _ => Err(Error::PrimitiveError("expected value".to_string())),
        }
    }
}

/// Handles function and operation calls.
#[derive(Debug, Clone, Copy, Default)]
pub struct CallHandler;

impl ExprHandler for CallHandler {
    fn can_handle(&self, expr: &RuntimeExpr) -> bool {
        matches!(expr, Expr::Call { .. })
    }

    fn eval(
        &self,
        expr: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
        host: &dyn Host<NonFunctionValue>,
        dispatch: &mut dyn FnMut(
            &RuntimeExpr,
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let Expr::Call {
            name,
            args,
            type_args,
        } = expr
        else {
            return Err(Error::PrimitiveError("expected call".to_string()));
        };

        ctx.increment_call_count()?;
        ctx.check_depth()?;

        if let Some(lazy) = host.lazy_function(name) {
            return lazy.call(args, ctx, dispatch);
        }

        let evaluated_args: Vec<NonFunctionValue> = args
            .iter()
            .map(|arg| dispatch(arg, ctx))
            .collect::<Result<Vec<_>, _>>()?;

        if let Some(operation) = host.operation(name) {
            return operation.perform(&evaluated_args, ctx);
        }

        if !type_args.is_empty() {
            if let Some(generic) = host.generic_function(name) {
                return generic.call_with_types(&evaluated_args, type_args, ctx);
            }
        }

        if let Some(function) = host.function(name) {
            return function.call(&evaluated_args, ctx);
        }

        if let Ok(value) = ctx.get(name)
            && FunctionValue::from_json(value.inner()).is_some()
        {
            let apply = host
                .function("apply")
                .ok_or_else(|| Error::UnknownFunction("apply".to_string()))?;
            let mut apply_args = vec![value];
            apply_args.extend(evaluated_args);
            return apply.call(&apply_args, ctx);
        }

        Err(Error::UnknownFunction(name.clone()))
    }
}

/// Handles calls to an arbitrary function value.
#[derive(Debug, Clone, Copy, Default)]
pub struct ApplyHandler;

impl ExprHandler for ApplyHandler {
    fn can_handle(&self, expr: &RuntimeExpr) -> bool {
        matches!(expr, Expr::Apply { .. })
    }

    fn eval(
        &self,
        expr: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
        host: &dyn Host<NonFunctionValue>,
        dispatch: &mut dyn FnMut(
            &RuntimeExpr,
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let Expr::Apply { target, args } = expr else {
            return Err(Error::PrimitiveError("expected apply".to_string()));
        };

        let fn_value = dispatch(target, ctx)?;
        let evaluated_args: Vec<NonFunctionValue> = args
            .iter()
            .map(|arg| dispatch(arg, ctx))
            .collect::<Result<Vec<_>, _>>()?;

        if let Some(name) = fn_value.as_str() {
            if let Some(function) = host.function(name) {
                return function.call(&evaluated_args, ctx);
            }
            return Err(Error::UnknownFunction(name.to_string()));
        }

        if FunctionValue::from_json(fn_value.inner()).is_none() {
            return Err(Error::TypeMismatch {
                expected: "function",
                got: "non-function",
            });
        }

        let apply = host
            .function("apply")
            .ok_or_else(|| Error::UnknownFunction("apply".to_string()))?;
        let mut apply_args = vec![fn_value];
        apply_args.extend(evaluated_args);
        apply.call(&apply_args, ctx)
    }
}

/// Handles array literals.
#[derive(Debug, Clone, Copy, Default)]
pub struct ArrayHandler;

impl ExprHandler for ArrayHandler {
    fn can_handle(&self, expr: &RuntimeExpr) -> bool {
        matches!(expr, Expr::Array(_))
    }

    fn eval(
        &self,
        expr: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
        _host: &dyn Host<NonFunctionValue>,
        dispatch: &mut dyn FnMut(
            &RuntimeExpr,
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let Expr::Array(elements) = expr else {
            return Err(Error::PrimitiveError("expected array".to_string()));
        };

        let mut values = Vec::with_capacity(elements.len());
        for elem in elements {
            values.push(dispatch(elem, ctx)?.into_inner());
        }
        Ok(NonFunctionValue::new(JsonValue::Array(values)))
    }
}

/// Handles object literals.
#[derive(Debug, Clone, Copy, Default)]
pub struct ObjectHandler;

impl ExprHandler for ObjectHandler {
    fn can_handle(&self, expr: &RuntimeExpr) -> bool {
        matches!(expr, Expr::Object(_))
    }

    fn eval(
        &self,
        expr: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
        _host: &dyn Host<NonFunctionValue>,
        dispatch: &mut dyn FnMut(
            &RuntimeExpr,
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let Expr::Object(entries) = expr else {
            return Err(Error::PrimitiveError("expected object".to_string()));
        };

        let mut map = serde_json::Map::with_capacity(entries.len());
        for (key, sub) in entries {
            let value = dispatch(sub, ctx)?.into_inner();
            map.insert(key.clone(), value);
        }
        Ok(NonFunctionValue::new(JsonValue::Object(map)))
    }
}

/// Handles postfix index expressions.
#[derive(Debug, Clone, Copy, Default)]
pub struct IndexHandler;

impl ExprHandler for IndexHandler {
    fn can_handle(&self, expr: &RuntimeExpr) -> bool {
        matches!(expr, Expr::Index { .. })
    }

    fn eval(
        &self,
        expr: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
        host: &dyn Host<NonFunctionValue>,
        dispatch: &mut dyn FnMut(
            &RuntimeExpr,
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let Expr::Index { target, index } = expr else {
            return Err(Error::PrimitiveError("expected index".to_string()));
        };

        let target_value = dispatch(target, ctx)?;
        let index_value = dispatch(index, ctx)?;

        let (name, got) = match target_value.inner() {
            JsonValue::Array(_) => ("array.at", "array"),
            JsonValue::Object(_) => ("obj.get", "object"),
            JsonValue::String(_) => ("str.at", "string"),
            other => {
                return Err(Error::TypeMismatch {
                    expected: "array, object, or string",
                    got: json_kind(other),
                });
            }
        };

        let Some(function) = host.function(name) else {
            return Err(Error::TypeMismatch {
                expected: "indexable",
                got,
            });
        };

        ctx.increment_call_count()?;
        ctx.check_depth()?;

        function.call(&[target_value, index_value], ctx)
    }
}

/// Handles postfix field access expressions.
#[derive(Debug, Clone, Copy, Default)]
pub struct FieldAccessHandler;

impl ExprHandler for FieldAccessHandler {
    fn can_handle(&self, expr: &RuntimeExpr) -> bool {
        matches!(expr, Expr::FieldAccess { .. })
    }

    fn eval(
        &self,
        expr: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
        _host: &dyn Host<NonFunctionValue>,
        dispatch: &mut dyn FnMut(
            &RuntimeExpr,
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let Expr::FieldAccess { target, field } = expr else {
            return Err(Error::PrimitiveError("expected field access".to_string()));
        };

        let target_value = dispatch(target, ctx)?;
        match target_value.inner() {
            JsonValue::Object(map) => {
                let value = map.get(field).cloned().unwrap_or(JsonValue::Null);
                Ok(NonFunctionValue::new(value))
            }
            _ => {
                if let Some(name) = qualified_name(target) {
                    let qualified = format!("{name}.{field}");
                    Ok(NonFunctionValue::new(JsonValue::String(qualified)))
                } else {
                    Err(Error::TypeMismatch {
                        expected: "object",
                        got: json_kind(target_value.inner()),
                    })
                }
            }
        }
    }
}

/// Handles effect-handler expressions.
#[derive(Debug, Clone, Copy, Default)]
pub struct HandleHandler;

impl ExprHandler for HandleHandler {
    fn can_handle(&self, expr: &RuntimeExpr) -> bool {
        matches!(expr, Expr::Handle { .. })
    }

    fn eval(
        &self,
        expr: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
        host: &dyn Host<NonFunctionValue>,
        _dispatch: &mut dyn FnMut(
            &RuntimeExpr,
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let result =
            crate::CanonicalVm::new(crate::HostRef::new(host)).eval_resumable(expr, ctx, None);

        match result {
            crate::EvalResult::Done(value) => Ok(value),
            crate::EvalResult::Yielded { value, .. } => {
                let name = value
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let args: Vec<NonFunctionValue> = value
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|v| NonFunctionValue::new(v.clone()))
                            .collect()
                    })
                    .unwrap_or_default();
                Err(Error::Operation {
                    name: name.to_string(),
                    args,
                })
            }
            crate::EvalResult::Error(err) => Err(err),
        }
    }
}

/// Handles bare identifier values.
#[derive(Debug, Clone, Copy, Default)]
pub struct VarHandler;

impl ExprHandler for VarHandler {
    fn can_handle(&self, expr: &RuntimeExpr) -> bool {
        matches!(expr, Expr::Var(_))
    }

    fn eval(
        &self,
        expr: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
        host: &dyn Host<NonFunctionValue>,
        _dispatch: &mut dyn FnMut(
            &RuntimeExpr,
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let Expr::Var(name) = expr else {
            return Err(Error::PrimitiveError("expected variable".to_string()));
        };

        match ctx.get(name) {
            Ok(value) => Ok(value),
            Err(_) => {
                if let Some(function) = host.function(name) {
                    function.call(&[], ctx)
                } else {
                    Err(Error::UnknownVariable(name.clone()))
                }
            }
        }
    }
}

/// Handles assignment expressions.
#[derive(Debug, Clone, Copy, Default)]
pub struct AssignHandler;

impl ExprHandler for AssignHandler {
    fn can_handle(&self, expr: &RuntimeExpr) -> bool {
        matches!(expr, Expr::Assign { .. })
    }

    fn eval(
        &self,
        expr: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
        host: &dyn Host<NonFunctionValue>,
        dispatch: &mut dyn FnMut(
            &RuntimeExpr,
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let Expr::Assign { target, value } = expr else {
            return Err(Error::PrimitiveError("expected assignment".to_string()));
        };

        for name in target.bound_names() {
            if host.is_reserved(&name) {
                return Err(Error::ReservedName(name));
            }
        }
        let evaluated = dispatch(value, ctx)?;
        define_pattern(ctx, target, evaluated.clone())?;
        Ok(evaluated)
    }
}

/// Handles first-class function literals.
#[derive(Debug, Clone, Copy, Default)]
pub struct FunctionHandler;

impl ExprHandler for FunctionHandler {
    fn can_handle(&self, expr: &RuntimeExpr) -> bool {
        matches!(expr, Expr::Function { .. })
    }

    fn eval(
        &self,
        expr: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
        _host: &dyn Host<NonFunctionValue>,
        dispatch: &mut dyn FnMut(
            &RuntimeExpr,
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let Expr::Function {
            type_params: _,
            params,
            ret,
            body,
        } = expr
        else {
            return Err(Error::PrimitiveError("expected function".to_string()));
        };

        let mut evaluated_params = Vec::new();
        let mut pattern_json = Vec::new();
        for (pattern, ty) in params {
            let schema = match ty {
                Some(type_expr) => dispatch(type_expr, ctx)?.into_inner(),
                None => JsonValue::Null,
            };
            for name in pattern.bound_names() {
                evaluated_params.push((name, schema.clone()));
            }
            pattern_json.push(pattern_to_json(pattern));
        }

        let return_schema = match ret {
            Some(type_expr) => Some(dispatch(type_expr, ctx)?.into_inner()),
            None => None,
        };

        let function_value = FunctionValue {
            params: evaluated_params,
            return_schema,
            body: body.clone(),
            patterns: pattern_json,
        };

        Ok(NonFunctionValue::new(function_value.to_json()))
    }
}

/// Builds a canonical `LegoVm` with the default expression handlers.
pub fn canonical_vm<H: Host<NonFunctionValue>>(host: H) -> LegoVm<H> {
    LegoVm::new(host)
        .with(ValueHandler)
        .with(VarHandler)
        .with(AssignHandler)
        .with(FunctionHandler)
        .with(CallHandler)
        .with(ApplyHandler)
        .with(ArrayHandler)
        .with(ObjectHandler)
        .with(IndexHandler)
        .with(FieldAccessHandler)
        .with(HandleHandler)
}
