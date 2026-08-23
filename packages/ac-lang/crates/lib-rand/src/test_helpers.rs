use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{CanonicalContext, Context, Error};

use super::module::RandModule;

pub fn call(name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = RandModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

pub fn call_err(name: &str, args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = RandModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx)
}

pub fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

pub fn arr(values: &[serde_json::Value]) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Array(values.to_vec()))
}

pub fn extract_num(v: &NonFunctionValue) -> f64 {
    v.as_number().expect("expected number")
}

pub fn extract_str(v: &NonFunctionValue) -> String {
    v.as_str().expect("expected string").to_string()
}

pub fn extract_array(v: &NonFunctionValue) -> Vec<serde_json::Value> {
    v.as_array().expect("expected array").to_vec()
}
