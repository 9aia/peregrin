use std::rc::Rc;

use handl_vm::{AsyncFunction, Function, GenericFunction, Host, LazyFunction, Operation};
use handl_vm_values_json::NonFunctionValue;

use crate::module::Module;

/// A composite host that dispatches function calls to registered modules.
///
/// `ModuleHost` implements [`Host`] by splitting the function name on the
/// first `.`. The left segment is the module name; the right segment is
/// the function name within that module. For example, `math.abs(x)` is
/// split into module `"math"` and function `"abs"`.
///
/// If the name contains no `.`, the host searches all modules in order
/// and returns the first match. This allows flat names (e.g. `add`) to
/// coexist with qualified names (e.g. `math.abs`).
///
/// # Architecture
///
/// ```text
///            ModuleHost
///                │
///                │ Host::function("math.abs")
///                ▼
///          split on "."
///           ┌────┴────┐
///           ▼         ▼
///      "math"     "abs"
///           │         │
///           ▼         ▼
///       Module    Function
/// ```
///
/// The VM sees only the `Host` interface — it does not know about modules.
#[derive(Clone)]
pub struct ModuleHost {
    modules: Vec<Rc<dyn Module>>,
}

impl ModuleHost {
    /// Creates a new empty host.
    pub fn new() -> Self {
        ModuleHost {
            modules: Vec::new(),
        }
    }

    /// Registers a module.
    pub fn with(mut self, module: impl Module + 'static) -> Self {
        self.modules.push(Rc::new(module));
        self
    }

    /// Returns the number of registered modules.
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }
}

impl Default for ModuleHost {
    fn default() -> Self {
        Self::new()
    }
}

impl Host<NonFunctionValue> for ModuleHost {
    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        if let Some((module_name, func_name)) = name.split_once('.') {
            self.modules
                .iter()
                .find(|m| m.name() == module_name)
                .and_then(|m| m.function(func_name))
        } else {
            self.modules.iter().find_map(|m| m.function(name))
        }
    }

    fn lazy_function(&self, name: &str) -> Option<&dyn LazyFunction<NonFunctionValue>> {
        if let Some((module_name, func_name)) = name.split_once('.') {
            self.modules
                .iter()
                .find(|m| m.name() == module_name)
                .and_then(|m| m.lazy_function(func_name))
        } else {
            self.modules.iter().find_map(|m| m.lazy_function(name))
        }
    }

    fn generic_function(&self, name: &str) -> Option<&dyn GenericFunction<NonFunctionValue>> {
        if let Some((module_name, func_name)) = name.split_once('.') {
            self.modules
                .iter()
                .find(|m| m.name() == module_name)
                .and_then(|m| m.generic_function(func_name))
        } else {
            self.modules.iter().find_map(|m| m.generic_function(name))
        }
    }

    fn async_function(&self, name: &str) -> Option<&dyn AsyncFunction<NonFunctionValue>> {
        if let Some((module_name, func_name)) = name.split_once('.') {
            self.modules
                .iter()
                .find(|m| m.name() == module_name)
                .and_then(|m| m.async_function(func_name))
        } else {
            self.modules.iter().find_map(|m| m.async_function(name))
        }
    }

    fn operation(&self, name: &str) -> Option<&dyn Operation<NonFunctionValue>> {
        if let Some((module_name, func_name)) = name.split_once('.') {
            self.modules
                .iter()
                .find(|m| m.name() == module_name)
                .and_then(|m| m.operation(func_name))
        } else {
            self.modules.iter().find_map(|m| m.operation(name))
        }
    }

    fn is_reserved(&self, name: &str) -> bool {
        if name.contains('.') {
            return false;
        }
        if self.modules.iter().any(|m| m.name() == name) {
            return true;
        }
        self.function(name).is_some()
            || self.lazy_function(name).is_some()
            || self.generic_function(name).is_some()
            || self.async_function(name).is_some()
            || self.operation(name).is_some()
    }
}
