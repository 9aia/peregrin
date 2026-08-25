//! Composite `TypeHost` bundling the standard library helpers and type helpers.

use handl_lib_utils::{ArrayModule, BoolModule, FnModule, ObjModule, StrModule};
use handl_vm::{Function, Host};
use handl_vm_values_json::NonFunctionValue;

use handl_vm_common::ModuleHost;
use handl_vm_host_canonical::CanonicalHost;

use super::functions::{
    AppArgsCompatible, FuncParamsCompatible, ObjEvery, PairSecond, TypeKind, TypeName, ZipAll,
};

/// A host that provides type-checking helper functions alongside
/// the standard `lib-utils` modules and default `var`/`resolve`.
#[derive(Clone)]
pub struct TypeHost {
    inner: ModuleHost,
    default: CanonicalHost,
}

impl TypeHost {
    pub fn new() -> Self {
        let inner = ModuleHost::new()
            .with(BoolModule)
            .with(StrModule)
            .with(ObjModule)
            .with(ArrayModule)
            .with(FnModule)
            .with(TypeHelpersModule);
        TypeHost {
            inner,
            default: CanonicalHost::default(),
        }
    }
}

impl Default for TypeHost {
    fn default() -> Self {
        Self::new()
    }
}

impl Host<NonFunctionValue> for TypeHost {
    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        self.inner
            .function(name)
            .or_else(|| self.default.function(name))
    }

    fn lazy_function(&self, name: &str) -> Option<&dyn handl_vm::LazyFunction<NonFunctionValue>> {
        self.inner.lazy_function(name)
    }

    fn generic_function(
        &self,
        name: &str,
    ) -> Option<&dyn handl_vm::GenericFunction<NonFunctionValue>> {
        self.inner.generic_function(name)
    }
}

/// Module containing type-checking helper host functions.
#[derive(Clone, Copy)]
pub struct TypeHelpersModule;

impl handl_vm_common::Module for TypeHelpersModule {
    fn name(&self) -> &str {
        "type"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "kind" => Some(&TypeKind),
            "name" => Some(&TypeName),
            "pair_second" => Some(&PairSecond),
            "zip_all" => Some(&ZipAll),
            "obj_every" => Some(&ObjEvery),
            "func_params_compatible" => Some(&FuncParamsCompatible),
            "app_args_compatible" => Some(&AppArgsCompatible),
            _ => None,
        }
    }
}
