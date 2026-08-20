use ac_runtime::RuntimeValue;
use ac_vm::{Context, Error, Function};

#[derive(Clone)]
pub struct Add;
impl Function for Add {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let sum: f64 = args.iter().filter_map(|v| v.as_number()).sum();
        Ok(RuntimeValue::new(serde_json::json!(sum)))
    }
}

#[derive(Clone)]
pub struct Mul;
impl Function for Mul {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let product = args
            .iter()
            .filter_map(|v| v.as_number())
            .product::<f64>();
        Ok(RuntimeValue::new(serde_json::json!(product)))
    }
}

#[derive(Clone)]
pub struct Concat;
impl Function for Concat {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let mut result = String::new();
        for arg in args {
            match arg.inner() {
                serde_json::Value::String(s) => result.push_str(s),
                _ => {
                    return Err(Error::TypeMismatch {
                        expected: "string",
                        got: "non-string",
                    })
                }
            }
        }
        Ok(RuntimeValue::new(result))
    }
}

#[derive(Clone)]
pub struct Len;
impl Function for Len {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let arg = args
            .get(0)
            .ok_or_else(|| Error::ArgumentCount { expected: 1, got: 0 })?;
        let len = match arg.inner() {
            serde_json::Value::String(s) => s.len(),
            serde_json::Value::Array(a) => a.len(),
            serde_json::Value::Object(o) => o.len(),
            _ => {
                return Err(Error::TypeMismatch {
                    expected: "string, array, or object",
                    got: "other",
                })
            }
        };
        Ok(RuntimeValue::new(serde_json::json!(len)))
    }
}
