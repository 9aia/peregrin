use ac_runtime::{Context, Error, Expr, Host, NonFunctionValue};
use ac_runtime_common::{ExprHandler, LegoVm};
use serde_json::Value as JsonValue;

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
        _ctx: &mut dyn Context,
        _host: &dyn Host,
        _dispatch: &mut dyn FnMut(&RuntimeExpr, &mut dyn Context) -> Result<NonFunctionValue, Error>,
    ) -> Result<NonFunctionValue, Error> {
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
        ctx: &mut dyn Context,
        host: &dyn Host,
        dispatch: &mut dyn FnMut(&RuntimeExpr, &mut dyn Context) -> Result<NonFunctionValue, Error>,
    ) -> Result<NonFunctionValue, Error> {
        let Expr::Call { name, args, type_args } = expr else {
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

        if let Ok(value) = ctx.get(name) && value.is_function() {
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
        ctx: &mut dyn Context,
        _host: &dyn Host,
        dispatch: &mut dyn FnMut(&RuntimeExpr, &mut dyn Context) -> Result<NonFunctionValue, Error>,
    ) -> Result<NonFunctionValue, Error> {
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
        ctx: &mut dyn Context,
        _host: &dyn Host,
        dispatch: &mut dyn FnMut(&RuntimeExpr, &mut dyn Context) -> Result<NonFunctionValue, Error>,
    ) -> Result<NonFunctionValue, Error> {
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
        ctx: &mut dyn Context,
        host: &dyn Host,
        _dispatch: &mut dyn FnMut(&RuntimeExpr, &mut dyn Context) -> Result<NonFunctionValue, Error>,
    ) -> Result<NonFunctionValue, Error> {
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
                    .map(|arr| arr.iter().map(|v| NonFunctionValue::new(v.clone())).collect())
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

/// Builds a canonical `LegoVm` with the default expression handlers.
pub fn canonical_vm<H: Host>(host: H) -> LegoVm<H> {
    LegoVm::new(host)
        .with(ValueHandler)
        .with(CallHandler)
        .with(ArrayHandler)
        .with(ObjectHandler)
        .with(HandleHandler)
}
