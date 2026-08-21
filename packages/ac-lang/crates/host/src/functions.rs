use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

#[derive(Clone)]
pub struct Add;
impl Function for Add {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let sum: f64 = args.iter().filter_map(|v| v.as_number()).sum();
        Ok(NonFunctionValue::new(serde_json::json!(sum)))
    }
}

#[derive(Clone)]
pub struct Mul;
impl Function for Mul {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let product = args.iter().filter_map(|v| v.as_number()).product::<f64>();
        Ok(NonFunctionValue::new(serde_json::json!(product)))
    }
}

#[derive(Clone)]
pub struct Concat;
impl Function for Concat {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let mut result = String::new();
        for arg in args {
            match arg.inner() {
                serde_json::Value::String(s) => result.push_str(s),
                _ => {
                    return Err(Error::TypeMismatch {
                        expected: "string",
                        got: "non-string",
                    });
                }
            }
        }
        Ok(NonFunctionValue::new(result))
    }
}

#[derive(Clone)]
pub struct Len;
impl Function for Len {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let arg = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        let len = match arg.inner() {
            serde_json::Value::String(s) => s.len(),
            serde_json::Value::Array(a) => a.len(),
            serde_json::Value::Object(o) => o.len(),
            _ => {
                return Err(Error::TypeMismatch {
                    expected: "string, array, or object",
                    got: "other",
                });
            }
        };
        Ok(NonFunctionValue::new(serde_json::json!(len)))
    }
}
