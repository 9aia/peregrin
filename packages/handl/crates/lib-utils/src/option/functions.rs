use handl_vm::{Context, Function};
use handl_vm_values_json::{Error, NonFunctionValue};
use serde_json::{Value as JsonValue, json};

use crate::shared::{apply_fn, is_truthy_value, require_function_value};

fn as_option_tag(value: &JsonValue) -> Option<&str> {
    value.get("_tag")?.as_str()
}

fn is_some(value: &JsonValue) -> bool {
    as_option_tag(value) == Some("Some")
}

fn is_none(value: &JsonValue) -> bool {
    as_option_tag(value) == Some("None")
}

fn require_option(arg: &NonFunctionValue) -> Result<(), Error> {
    let inner = arg.inner();
    if as_option_tag(inner).is_some() {
        Ok(())
    } else {
        Err(Error::TypeMismatch {
            expected: "option",
            got: "non-option",
        })
    }
}

fn get_value(arg: &NonFunctionValue) -> Option<&JsonValue> {
    arg.inner().get("value")
}

#[derive(Clone, Copy)]
pub struct SomeFn;
impl Function<NonFunctionValue> for SomeFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let arg = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        Ok(NonFunctionValue::new(json!({
            "_tag": "Some",
            "value": arg.inner().clone(),
        })))
    }
}

#[derive(Clone, Copy)]
pub struct NoneFn;
impl Function<NonFunctionValue> for NoneFn {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        Ok(NonFunctionValue::new(json!({"_tag": "None"})))
    }
}

#[derive(Clone, Copy)]
pub struct IsSome;
impl Function<NonFunctionValue> for IsSome {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let arg = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        Ok(NonFunctionValue::new(json!(is_some(arg.inner()))))
    }
}

#[derive(Clone, Copy)]
pub struct IsNone;
impl Function<NonFunctionValue> for IsNone {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let arg = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        Ok(NonFunctionValue::new(json!(is_none(arg.inner()))))
    }
}

#[derive(Clone, Copy)]
pub struct Unwrap;
impl Function<NonFunctionValue> for Unwrap {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let arg = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        require_option(arg)?;
        if let Some(value) = get_value(arg) {
            Ok(NonFunctionValue::new(value.clone()))
        } else {
            Err(Error::PrimitiveError("called unwrap on None".to_string()))
        }
    }
}

#[derive(Clone, Copy)]
pub struct UnwrapOr;
impl Function<NonFunctionValue> for UnwrapOr {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        require_option(&args[0])?;
        if let Some(value) = get_value(&args[0]) {
            Ok(NonFunctionValue::new(value.clone()))
        } else {
            Ok(args[1].clone())
        }
    }
}

#[derive(Clone, Copy)]
pub struct UnwrapOrElse;
impl Function<NonFunctionValue> for UnwrapOrElse {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        require_option(&args[0])?;
        if let Some(value) = get_value(&args[0]) {
            Ok(NonFunctionValue::new(value.clone()))
        } else {
            let fn_val = require_function_value(&args[1])?;
            ctx.eval_expr(&fn_val.body, &[])
        }
    }
}

#[derive(Clone, Copy)]
pub struct Map;
impl Function<NonFunctionValue> for Map {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        require_option(&args[0])?;
        let fn_val = require_function_value(&args[1])?;
        let value = get_value(&args[0]).cloned().unwrap_or(JsonValue::Null);
        if as_option_tag(args[0].inner()) == Some("Some") {
            let mapped = apply_fn(ctx, &fn_val, &value)?;
            Ok(NonFunctionValue::new(json!({
                "_tag": "Some",
                "value": mapped.inner().clone(),
            })))
        } else {
            Ok(NonFunctionValue::new(json!({"_tag": "None"})))
        }
    }
}

#[derive(Clone, Copy)]
pub struct FlatMap;
impl Function<NonFunctionValue> for FlatMap {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        require_option(&args[0])?;
        let fn_val = require_function_value(&args[1])?;
        let value = get_value(&args[0]).cloned().unwrap_or(JsonValue::Null);
        if as_option_tag(args[0].inner()) == Some("Some") {
            apply_fn(ctx, &fn_val, &value)
        } else {
            Ok(NonFunctionValue::new(json!({"_tag": "None"})))
        }
    }
}

#[derive(Clone, Copy)]
pub struct Filter;
impl Function<NonFunctionValue> for Filter {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        require_option(&args[0])?;
        let fn_val = require_function_value(&args[1])?;
        if as_option_tag(args[0].inner()) == Some("Some") {
            let value = get_value(&args[0]).cloned().unwrap_or(JsonValue::Null);
            let pred = apply_fn(ctx, &fn_val, &value)?;
            if is_truthy_value(&pred) {
                Ok(args[0].clone())
            } else {
                Ok(NonFunctionValue::new(json!({"_tag": "None"})))
            }
        } else {
            Ok(NonFunctionValue::new(json!({"_tag": "None"})))
        }
    }
}

#[derive(Clone, Copy)]
pub struct MapOr;
impl Function<NonFunctionValue> for MapOr {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        require_option(&args[0])?;
        let fn_val = require_function_value(&args[2])?;
        let value = get_value(&args[0]).cloned().unwrap_or(JsonValue::Null);
        if as_option_tag(args[0].inner()) == Some("Some") {
            apply_fn(ctx, &fn_val, &value)
        } else {
            Ok(args[1].clone())
        }
    }
}

#[derive(Clone, Copy)]
pub struct Or;
impl Function<NonFunctionValue> for Or {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        if as_option_tag(args[0].inner()) == Some("Some") {
            Ok(args[0].clone())
        } else {
            Ok(args[1].clone())
        }
    }
}

#[derive(Clone, Copy)]
pub struct OrElse;
impl Function<NonFunctionValue> for OrElse {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        if as_option_tag(args[0].inner()) == Some("Some") {
            Ok(args[0].clone())
        } else {
            let fn_val = require_function_value(&args[1])?;
            ctx.eval_expr(&fn_val.body, &[])
        }
    }
}

#[derive(Clone, Copy)]
pub struct And;
impl Function<NonFunctionValue> for And {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        if as_option_tag(args[0].inner()) == Some("Some") {
            Ok(args[1].clone())
        } else {
            Ok(NonFunctionValue::new(json!({"_tag": "None"})))
        }
    }
}

#[derive(Clone, Copy)]
pub struct Xor;
impl Function<NonFunctionValue> for Xor {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let a_is_some = is_some(args[0].inner());
        let b_is_some = is_some(args[1].inner());
        if a_is_some && !b_is_some {
            Ok(args[0].clone())
        } else if !a_is_some && b_is_some {
            Ok(args[1].clone())
        } else {
            Ok(NonFunctionValue::new(json!({"_tag": "None"})))
        }
    }
}
