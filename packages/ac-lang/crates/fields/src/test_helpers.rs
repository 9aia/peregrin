use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function, Host};
use serde_json::Value as JsonValue;

pub use ac_runtime_example_1::DefaultHost;

/// A `Host` that extends [`DefaultHost`] with a `double` helper for
/// tests that exercise user-defined function values.
#[derive(Clone)]
pub struct MathHost {
    default: DefaultHost,
    double: Double}

impl MathHost {
    pub fn new() -> Self {
        MathHost {
            default: DefaultHost::default(),
            double: Double}
    }
}

impl Default for MathHost {
    fn default() -> Self {
        Self::new()
    }
}

impl Host for MathHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        if name == "double" {
            return Some(&self.double);
        }
        self.default.function(name)
    }
}

#[derive(Clone)]
pub struct Double;
impl Function for Double {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let x = args
            .first()
            .and_then(|v| v.as_number())
            .ok_or(Error::ArgumentCount { expected: 1, got: 0 })?;
        Ok(NonFunctionValue::new(x * 2.0))
    }
}

pub fn ns_with_doc(data: &JsonValue) -> crate::types::Namespaces {
    let mut ns = crate::types::Namespaces::new();
    ns.insert("doc".to_string(), data.clone());
    ns
}
