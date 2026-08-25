use crate::pattern::{define_pattern, pattern_to_json};
use crate::{Context, Error, Expr, Host, VirtualMachine};
use handl_vm_values_json::{FunctionValue, NonFunctionValue};
use serde_json::Value as JsonValue;

type RuntimeExpr = Expr<NonFunctionValue>;

pub(crate) fn json_kind(v: &JsonValue) -> &'static str {
    match v {
        JsonValue::Array(_) => "array",
        JsonValue::Object(_) => "object",
        JsonValue::String(_) => "string",
        JsonValue::Number(_) => "number",
        JsonValue::Bool(_) => "boolean",
        JsonValue::Null => "null",
    }
}

pub(crate) fn qualified_name(expr: &RuntimeExpr) -> Option<String> {
    match expr {
        Expr::Var(name) => Some(name.clone()),
        Expr::FieldAccess { target, field } => {
            qualified_name(target).map(|name| format!("{name}.{field}"))
        }
        _ => None,
    }
}

/// The language virtual machine.
///
/// The VM evaluates expressions by walking the expression tree.
/// Values are returned as-is, calls are dispatched to host-provided
/// functions via the [`Host`] trait.
///
/// # Type parameters
///
/// - `H` — the host type, which must implement [`Host`].
pub struct CanonicalVm<H: Host<NonFunctionValue>> {
    host: H,
}

impl<H: Host<NonFunctionValue>> CanonicalVm<H> {
    /// Creates a new VM with the given host.
    pub fn new(host: H) -> Self {
        CanonicalVm { host }
    }

    /// Returns a reference to the host.
    pub fn host(&self) -> &H {
        &self.host
    }

    /// Evaluates an expression in the given context.
    ///
    /// This is the core evaluation loop. It recursively evaluates
    /// sub-expressions, dispatches calls to host functions, and
    /// returns the resulting [`NonFunctionValue`].
    ///
    /// For `Call` expressions, the VM checks [`Host::lazy_function`]
    /// first. If the name resolves to a lazy function, the VM passes
    /// the unevaluated argument expressions and an evaluate closure.
    /// The function decides which arguments to evaluate (short-circuit).
    /// Otherwise, the VM eagerly evaluates all arguments and dispatches
    /// to [`Host::function`] or [`Host::operation`].
    pub fn eval(
        &self,
        expr: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        match expr {
            Expr::Value(value) => Ok(value.clone()),

            Expr::Var(name) => match ctx.get(name) {
                Ok(value) => Ok(value),
                Err(_) => {
                    if let Some(function) = self.host.function(name) {
                        function.call(&[], ctx)
                    } else {
                        Err(Error::UnknownVariable(name.clone()))
                    }
                }
            },

            Expr::Assign { target, value } => {
                for name in target.bound_names() {
                    if self.host.is_reserved(&name) {
                        return Err(Error::ReservedName(name));
                    }
                }
                let evaluated = self.eval(value, ctx)?;
                define_pattern(ctx, target, evaluated.clone())?;
                Ok(evaluated)
            }

            Expr::Function {
                type_params: _,
                params,
                ret,
                body,
            } => {
                let mut evaluated_params = Vec::new();
                let mut pattern_json = Vec::new();
                for (pattern, ty) in params {
                    let schema = match ty {
                        Some(type_expr) => self.eval(type_expr, ctx)?.into_inner(),
                        None => JsonValue::Null,
                    };
                    for name in pattern.bound_names() {
                        evaluated_params.push((name, schema.clone()));
                    }
                    pattern_json.push(pattern_to_json(pattern));
                }

                let return_schema = match ret {
                    Some(type_expr) => Some(self.eval(type_expr, ctx)?.into_inner()),
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

            Expr::Handle { .. } => {
                // Handle expressions use the resumable evaluation
                // machinery internally. We start a resumable eval
                // and run it to completion (handlers intercept
                // operations inline, so no yielding should occur
                // unless an operation has no matching handler).
                let result = self.eval_resumable(expr, ctx, None);
                match result {
                    crate::EvalResult::Done(value) => Ok(value),
                    crate::EvalResult::Yielded { value, .. } => {
                        // An operation propagated past all handlers.
                        // Extract the operation name and args.
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

            Expr::Call {
                name,
                args,
                type_args,
            } => {
                ctx.increment_call_count()?;
                ctx.check_depth()?;

                // Check lazy functions first — they receive unevaluated
                // args and decide which to evaluate (short-circuit).
                if let Some(lazy) = self.host.lazy_function(name) {
                    let mut evaluate =
                        |sub: &RuntimeExpr, c: &mut dyn Context<NonFunctionValue>| {
                            self.eval(sub, c)
                        };
                    return lazy.call(args, ctx, &mut evaluate);
                }

                let evaluated_args = args
                    .iter()
                    .map(|arg| self.eval(arg, ctx))
                    .collect::<Result<Vec<NonFunctionValue>, Error<NonFunctionValue>>>()?;

                if let Some(operation) = self.host.operation(name) {
                    return operation.perform(&evaluated_args, ctx);
                }

                // If the call carries type arguments, try dispatching to
                // a GenericFunction that can inspect them.
                if !type_args.is_empty() {
                    if let Some(generic) = self.host.generic_function(name) {
                        return generic.call_with_types(&evaluated_args, type_args, ctx);
                    }
                }

                if let Some(function) = self.host.function(name) {
                    return function.call(&evaluated_args, ctx);
                }

                if let Ok(value) = ctx.get(name)
                    && FunctionValue::from_json(value.inner()).is_some()
                {
                    let apply = self
                        .host
                        .function("apply")
                        .ok_or_else(|| Error::UnknownFunction("apply".to_string()))?;
                    let mut apply_args = vec![value];
                    apply_args.extend(evaluated_args);
                    return apply.call(&apply_args, ctx);
                }

                Err(Error::UnknownFunction(name.clone()))
            }

            Expr::Apply { target, args } => {
                ctx.increment_call_count()?;
                ctx.check_depth()?;

                let fn_value = self.eval(target, ctx)?;
                let evaluated_args: Vec<NonFunctionValue> = args
                    .iter()
                    .map(|arg| self.eval(arg, ctx))
                    .collect::<Result<Vec<_>, _>>()?;

                if let Some(name) = fn_value.as_str() {
                    if let Some(function) = self.host.function(name) {
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

                let apply = self
                    .host
                    .function("apply")
                    .ok_or_else(|| Error::UnknownFunction("apply".to_string()))?;
                let mut apply_args = vec![fn_value];
                apply_args.extend(evaluated_args);
                apply.call(&apply_args, ctx)
            }

            Expr::Index { target, index } => {
                let target_value = self.eval(target, ctx)?;
                let index_value = self.eval(index, ctx)?;

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

                let Some(function) = self.host.function(name) else {
                    return Err(Error::TypeMismatch {
                        expected: "indexable",
                        got,
                    });
                };

                ctx.increment_call_count()?;
                ctx.check_depth()?;

                function.call(&[target_value, index_value], ctx)
            }

            Expr::FieldAccess { target, field } => {
                let target_value = self.eval(target, ctx)?;
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

            Expr::Array(elements) => {
                let mut values = Vec::with_capacity(elements.len());
                for elem in elements {
                    values.push(self.eval(elem, ctx)?.into_inner());
                }
                Ok(NonFunctionValue::new(JsonValue::Array(values)))
            }

            Expr::Object(entries) => {
                let mut map = serde_json::Map::with_capacity(entries.len());
                for (key, expr) in entries {
                    let value = self.eval(expr, ctx)?.into_inner();
                    map.insert(key.clone(), value);
                }
                Ok(NonFunctionValue::new(JsonValue::Object(map)))
            }
        }
    }

    /// Evaluates an expression asynchronously.
    ///
    /// Like [`eval`](CanonicalVm::eval), but supports async host functions.
    /// The VM checks [`Host::function`] first (sync), then falls back
    /// to [`Host::async_function`] (async). This means a host can
    /// provide both sync and async functions, and the VM will use
    /// whichever is available.
    ///
    /// # Yielding
    ///
    /// Async host functions can yield control to the executor by
    /// awaiting a pending future. The entire evaluation suspends
    /// until the future is ready.
    ///
    /// To explicitly yield a value to the caller, a host function
    /// returns `Err(Error::Yield(value))`. The caller receives
    /// `Err(Error::Yield(value))` and can inspect the yielded value.
    ///
    /// Like [`eval`](CanonicalVm::eval), this checks [`Host::lazy_function`]
    /// first for short-circuit evaluation.
    pub async fn eval_async(
        &self,
        expr: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        match expr {
            Expr::Value(value) => Ok(value.clone()),

            Expr::Var(name) => match ctx.get(name) {
                Ok(value) => Ok(value),
                Err(_) => {
                    if let Some(function) = self.host.function(name) {
                        function.call(&[], ctx)
                    } else {
                        Err(Error::UnknownVariable(name.clone()))
                    }
                }
            },

            Expr::Assign { target, value } => {
                for name in target.bound_names() {
                    if self.host.is_reserved(&name) {
                        return Err(Error::ReservedName(name));
                    }
                }
                let evaluated = Box::pin(self.eval_async(value, ctx)).await?;
                define_pattern(ctx, target, evaluated.clone())?;
                Ok(evaluated)
            }

            Expr::Function {
                type_params: _,
                params,
                ret,
                body,
            } => {
                let mut evaluated_params = Vec::new();
                let mut pattern_json = Vec::new();
                for (pattern, ty) in params {
                    let schema = match ty {
                        Some(type_expr) => Box::pin(self.eval_async(type_expr, ctx))
                            .await?
                            .into_inner(),
                        None => JsonValue::Null,
                    };
                    for name in pattern.bound_names() {
                        evaluated_params.push((name, schema.clone()));
                    }
                    pattern_json.push(pattern_to_json(pattern));
                }

                let return_schema = match ret {
                    Some(type_expr) => Some(
                        Box::pin(self.eval_async(type_expr, ctx))
                            .await?
                            .into_inner(),
                    ),
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

            Expr::Handle { .. } => {
                // Handle expressions use the resumable evaluation
                // machinery internally (same as sync eval).
                let result = self.eval_resumable(expr, ctx, None);
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

            Expr::Call {
                name,
                args,
                type_args,
            } => {
                ctx.increment_call_count()?;
                ctx.check_depth()?;

                // Check lazy functions first — they receive unevaluated
                // args and decide which to evaluate (short-circuit).
                if let Some(lazy) = self.host.lazy_function(name) {
                    let mut eval_sync =
                        |sub: &RuntimeExpr, c: &mut dyn Context<NonFunctionValue>| {
                            self.eval(sub, c)
                        };
                    return lazy.call(args, ctx, &mut eval_sync);
                }

                let mut evaluated_args = Vec::with_capacity(args.len());
                for arg in args {
                    let value = Box::pin(self.eval_async(arg, ctx)).await?;
                    evaluated_args.push(value);
                }

                if let Some(operation) = self.host.operation(name) {
                    return operation.perform(&evaluated_args, ctx);
                }

                // If the call carries type arguments, try dispatching to
                // a GenericFunction that can inspect them.
                if !type_args.is_empty() {
                    if let Some(generic) = self.host.generic_function(name) {
                        return generic.call_with_types(&evaluated_args, type_args, ctx);
                    }
                }

                if let Some(function) = self.host.function(name) {
                    return function.call(&evaluated_args, ctx);
                }

                if let Ok(value) = ctx.get(name)
                    && FunctionValue::from_json(value.inner()).is_some()
                {
                    let apply = self
                        .host
                        .function("apply")
                        .ok_or_else(|| Error::UnknownFunction("apply".to_string()))?;
                    let mut apply_args = vec![value];
                    apply_args.extend(evaluated_args);
                    return apply.call(&apply_args, ctx);
                }

                if let Some(async_function) = self.host.async_function(name) {
                    return async_function.call(&evaluated_args, ctx).await;
                }

                Err(Error::UnknownFunction(name.clone()))
            }

            Expr::Index { target, index } => {
                let target_value = Box::pin(self.eval_async(target, ctx)).await?;
                let index_value = Box::pin(self.eval_async(index, ctx)).await?;

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

                let Some(function) = self.host.function(name) else {
                    return Err(Error::TypeMismatch {
                        expected: "indexable",
                        got,
                    });
                };

                ctx.increment_call_count()?;
                ctx.check_depth()?;

                function.call(&[target_value, index_value], ctx)
            }

            Expr::FieldAccess { target, field } => {
                let target_value = Box::pin(self.eval_async(target, ctx)).await?;
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

            Expr::Apply { target, args } => {
                ctx.increment_call_count()?;
                ctx.check_depth()?;

                let fn_value = Box::pin(self.eval_async(target, ctx)).await?;

                let mut evaluated_args = Vec::with_capacity(args.len());
                for arg in args {
                    let value = Box::pin(self.eval_async(arg, ctx)).await?;
                    evaluated_args.push(value);
                }

                if let Some(name) = fn_value.as_str() {
                    if let Some(function) = self.host.function(name) {
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

                let apply = self
                    .host
                    .function("apply")
                    .ok_or_else(|| Error::UnknownFunction("apply".to_string()))?;
                let mut apply_args = vec![fn_value];
                apply_args.extend(evaluated_args);
                apply.call(&apply_args, ctx)
            }

            Expr::Array(elements) => {
                let mut values = Vec::with_capacity(elements.len());
                for elem in elements {
                    values.push(Box::pin(self.eval_async(elem, ctx)).await?.into_inner());
                }
                Ok(NonFunctionValue::new(JsonValue::Array(values)))
            }

            Expr::Object(entries) => {
                let mut map = serde_json::Map::with_capacity(entries.len());
                for (key, expr) in entries {
                    let value = Box::pin(self.eval_async(expr, ctx)).await?.into_inner();
                    map.insert(key.clone(), value);
                }
                Ok(NonFunctionValue::new(JsonValue::Object(map)))
            }
        }
    }
}

impl<H: Host<NonFunctionValue>> VirtualMachine<NonFunctionValue> for CanonicalVm<H> {
    fn eval(
        &self,
        expr: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        Self::eval(self, expr, ctx)
    }

    fn eval_async(
        &self,
        expr: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> impl std::future::Future<Output = Result<NonFunctionValue, Error<NonFunctionValue>>> {
        Self::eval_async(self, expr, ctx)
    }
}
