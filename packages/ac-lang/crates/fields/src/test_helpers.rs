use crate::expression::eval_body;
use crate::types::{APPLY_FUNCTION, FunctionResolver, Namespaces};
use ac_dialect::Dialect;
use ac_runtime::FunctionValue;
use serde_json::Value as JsonValue;

pub struct MulResolver;
impl FunctionResolver for MulResolver {
    fn call(&self, name: &str, args: &[JsonValue]) -> Result<JsonValue, String> {
        match name {
            "mul" => {
                let product: f64 = args.iter().filter_map(|v| v.as_f64()).product();
                serde_json::Number::from_f64(product)
                    .map(JsonValue::Number)
                    .ok_or("NaN".to_string())
            }
            _ => Err(format!("unknown function: {name}")),
        }
    }
}

pub struct AddResolver;
impl FunctionResolver for AddResolver {
    fn call(&self, name: &str, args: &[JsonValue]) -> Result<JsonValue, String> {
        match name {
            "add" => {
                let sum: f64 = args.iter().filter_map(|v| v.as_f64()).sum();
                serde_json::Number::from_f64(sum)
                    .map(JsonValue::Number)
                    .ok_or("NaN".to_string())
            }
            _ => Err(format!("unknown function: {name}")),
        }
    }
}

pub struct ConcatResolver;
impl FunctionResolver for ConcatResolver {
    fn call(&self, name: &str, args: &[JsonValue]) -> Result<JsonValue, String> {
        match name {
            "concat" => {
                let s: String = args.iter().filter_map(|v| v.as_str()).collect();
                Ok(JsonValue::String(s))
            }
            _ => Err(format!("unknown function: {name}")),
        }
    }
}

pub struct MathResolver;
impl FunctionResolver for MathResolver {
    fn call(&self, name: &str, args: &[JsonValue]) -> Result<JsonValue, String> {
        match name {
            "add" => {
                let sum: f64 = args.iter().filter_map(|v| v.as_f64()).sum();
                serde_json::Number::from_f64(sum)
                    .map(JsonValue::Number)
                    .ok_or("NaN".to_string())
            }
            "mul" => {
                let product: f64 = args.iter().filter_map(|v| v.as_f64()).product();
                serde_json::Number::from_f64(product)
                    .map(JsonValue::Number)
                    .ok_or("NaN".to_string())
            }
            "double" => {
                let x = args[0].as_f64().unwrap();
                serde_json::Number::from_f64(x * 2.0)
                    .map(JsonValue::Number)
                    .ok_or("NaN".to_string())
            }
            _ => Err(format!("unknown function: {name}")),
        }
    }
}

pub struct LenResolver;
impl FunctionResolver for LenResolver {
    fn call(&self, name: &str, args: &[JsonValue]) -> Result<JsonValue, String> {
        match name {
            "len" => Ok(JsonValue::Number(serde_json::Number::from(
                args[0].as_str().unwrap().len() as i64,
            ))),
            _ => Err(format!("unknown function: {name}")),
        }
    }
}

pub struct DoubleResolver;
impl FunctionResolver for DoubleResolver {
    fn call(&self, name: &str, args: &[JsonValue]) -> Result<JsonValue, String> {
        match name {
            "double" => {
                let x = args[0].as_f64().unwrap();
                serde_json::Number::from_f64(x * 2.0)
                    .map(JsonValue::Number)
                    .ok_or("NaN".to_string())
            }
            _ => Err(format!("unknown function: {name}")),
        }
    }
}

#[allow(dead_code)]
pub fn ns_with_doc(data: &JsonValue) -> Namespaces {
    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), data.clone());
    ns
}

/// A `FunctionResolver` wrapper that adds `apply` support to any
/// inner resolver. When `apply` is called, it extracts the function
/// value (object with `body` field) from the first argument, binds
/// params to remaining args, and evaluates the body via `eval_body`.
/// All other function names delegate to the inner resolver.
pub struct ApplyResolver<'a, R: FunctionResolver> {
    pub inner: &'a R,
    pub namespaces: &'a Namespaces,
    pub dialect: &'a dyn Dialect,
}

impl<'a, R: FunctionResolver> FunctionResolver for ApplyResolver<'a, R> {
    fn call(&self, name: &str, args: &[JsonValue]) -> Result<JsonValue, String> {
        if name == APPLY_FUNCTION {
            let fn_value = args
                .first()
                .ok_or("apply() requires at least one argument (the function value)")?;
            let fn_val = FunctionValue::from_json(fn_value)
                .ok_or("apply(): first argument is not a function value")?;

            let fn_args = &args[1..];
            let params: Vec<(String, JsonValue)> = fn_val
                .params
                .iter()
                .zip(fn_args.iter())
                .map(|((pname, _), arg)| (pname.clone(), arg.clone()))
                .collect();

            eval_body(&fn_val.body, &params, self.namespaces, self, self.dialect)
                .ok_or_else(|| "apply(): failed to evaluate function body".to_string())
        } else {
            self.inner.call(name, args)
        }
    }
}
