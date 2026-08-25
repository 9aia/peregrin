use handl_vm::{Context, Function, Host};
use handl_vm_common::Module;
use handl_vm_host_canonical::CanonicalHost as BaseHost;
use handl_vm_values_json::Error;
use handl_vm_values_json::NonFunctionValue;
use serde_json::Value as JsonValue;

/// A test host that mirrors the builtins no longer in `handl_vm_host_canonical::CanonicalHost`
/// by delegating arithmetic and string operations to `handl-lib-utils` modules.
#[derive(Clone)]
pub struct FieldsTestHost {
    base: BaseHost,
    math: handl_lib_utils::MathModule,
    str: handl_lib_utils::StrModule,
}

impl Default for FieldsTestHost {
    fn default() -> Self {
        FieldsTestHost {
            base: BaseHost::default(),
            math: handl_lib_utils::MathModule,
            str: handl_lib_utils::StrModule,
        }
    }
}

impl Host<NonFunctionValue> for FieldsTestHost {
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

/// A `Host` that extends [`FieldsTestHost`] with a `double` helper for
/// tests that exercise user-defined function values.
#[derive(Clone)]
pub struct MathHost {
    default: FieldsTestHost,
    double: Double,
}

impl MathHost {
    pub fn new() -> Self {
        MathHost {
            default: FieldsTestHost::default(),
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
