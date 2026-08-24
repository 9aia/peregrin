use ac_runtime::{Context, Function, Host};
use ac_runtime_modules::Module;
use ac_runtime_values::Error;
use ac_runtime_values::NonFunctionValue;
use serde_json::Value as JsonValue;

use ac_runtime_example_1::DefaultHost as BaseHost;

/// A test host that mirrors the builtins no longer in `ac_runtime_example_1::DefaultHost`
/// by delegating arithmetic and string operations to `ac-lib-utils` modules.
#[derive(Clone)]
pub struct DefaultHost {
    base: BaseHost,
    math: ac_lib_utils::MathModule,
    str: ac_lib_utils::StrModule,
}

impl Default for DefaultHost {
    fn default() -> Self {
        DefaultHost {
            base: BaseHost::default(),
            math: ac_lib_utils::MathModule,
            str: ac_lib_utils::StrModule,
        }
    }
}

impl Host<NonFunctionValue> for DefaultHost {
    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "add" => self.math.function("add"),
            "sub" => self.math.function("sub"),
            "mul" => self.math.function("mul"),
            "div" => self.math.function("div"),
            "concat" => self.str.function("concat"),
            "len" => self.str.function("length"),
            _ => self.base.function(name),
        }
    }
}

/// A `Host` that extends [`DefaultHost`] with a `double` helper for
/// tests that exercise user-defined function values.
#[derive(Clone)]
pub struct MathHost {
    default: DefaultHost,
    double: Double,
}

impl MathHost {
    pub fn new() -> Self {
        MathHost {
            default: DefaultHost::default(),
            double: Double,
        }
    }
}

impl Default for MathHost {
    fn default() -> Self {
        Self::new()
    }
}

impl Host<NonFunctionValue> for MathHost {
    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        if name == "double" {
            return Some(&self.double);
        }
        self.default.function(name)
    }
}

#[derive(Clone)]
pub struct Double;
impl Function<NonFunctionValue> for Double {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let x = args
            .first()
            .and_then(|v| v.as_number())
            .ok_or(Error::ArgumentCount {
                expected: 1,
                got: 0,
            })?;
        Ok(NonFunctionValue::new(x * 2.0))
    }
}

pub fn ns_with_doc(data: &JsonValue) -> crate::types::Namespaces {
    let mut ns = crate::types::Namespaces::new();
    ns.insert("doc".to_string(), data.clone());
    ns
}
