//! # handl-vm-config
//!
//! A [`Host`] for running untrusted handl expressions in a sandbox.
//!
//! Provides only pure (side-effect-free) library modules plus the built-in
//! [`CanonicalHost`] functions. No I/O, no side effects, no `import()`.
//! Resource limits (scope depth, call count) are enforced via the
//! [`Context`] returned by [`SandboxHost::new_context`].
//!
//! A host that provides only pure functions is automatically sandboxed —
//! this is a policy decision, not a language feature. The VM enforces
//! the resource limits; the host configures them.
//!
//! ## What's included
//!
//! - **Built-ins** (`add`, `mul`, `concat`, `len`, `var`, `resolve`, `apply`, `array`)
//! - **`math.*`**, **`str.*`**, **`array.*`**, **`set.*`**, **`bool.*`**,
//!   **`cmp.*`**, **`obj.*`**, **`fn.*`**, **`type.*`** — from [`handl-lib-utils`]
//! - **`crypto.*`** — SHA-256, MD5, Base64, hex — from [`handl-lib-crypto`]
//! - **`datetime.*`** — parsing, formatting, arithmetic — from [`handl-lib-datetime`]
//! - **`intl.*`** — locale-aware formatting (ICU4X) — from [`handl-lib-intl`]
//! - **`kv.*`** — in-memory key-value store — from [`handl-lib-kv`] + [`handl-lib-kv-hashmap`]
//!
//! ## What's excluded
//!
//! - [`handl-lib-rand`] — non-deterministic RNG
//! - [`handl-lib-os`] / [`handl-lib-os-std`] — filesystem, env,
//!   process spawning, system clock
//! - [`handl-lib-http-client`] / [`handl-lib-http-client-std`] — HTTP client
//!
//! ## Usage
//!
//! ```ignore
//! use handl_vm_config::{SandboxHost, SandboxLimits};
//! use handl_vm_backend_rust_interpreter::Vm;
//!
//! let host = SandboxHost::new(SandboxLimits::default());
//! let ctx = host.new_context();
//! let vm = Vm::new(host);
//! ```
//!
//! With custom limits:
//!
//! ```ignore
//! use handl_vm_config::{SandboxHost, SandboxLimits};
//!
//! let limits = SandboxLimits {
//!     max_depth: Some(50),
//!     max_calls: Some(1_000),
//! };
//! let host = SandboxHost::new(limits);
//! let ctx = host.new_context();
//! ```

use handl_lib_crypto::CryptoModule;
use handl_lib_datetime::DateTimeModule;
use handl_lib_intl::IntlModule;
use handl_lib_kv::KeyValueStoreModule;
use handl_lib_kv_hashmap::HashKeyValueStore;
use handl_lib_utils::{
    ArrayModule, BoolModule, CmpModule, FnModule, MathModule, ObjModule, SetModule, StrModule,
    TypeModule,
};
use handl_vm_backend_rust_interpreter::CanonicalEval;
use handl_vm_common::ModuleHost;
use handl_vm_host_canonical::CanonicalHost;
use handl_vm_values_json::{
    CanonicalContext, Function, GenericFunction, Host, LazyFunction, NonFunctionValue,
};

/// Resource limits for sandboxed evaluation.
///
/// Configured on the [`SandboxHost`] and applied to every [`Context`]
/// created via [`SandboxHost::new_context`]. The VM enforces these
/// limits during evaluation — see [`Error::ResourceLimit`](handl_vm::Error::ResourceLimit).
#[derive(Clone, Debug)]
pub struct SandboxLimits {
    /// Maximum scope depth (function call nesting).
    ///
    /// `None` means no limit. The global scope counts as depth 1,
    /// so `max_depth: Some(100)` allows 99 nested calls.
    pub max_depth: Option<usize>,

    /// Maximum number of function calls during a single evaluation.
    ///
    /// `None` means no limit. Each `Call` expression increments the
    /// counter; exceeding the limit returns
    /// [`Error::ResourceLimit`](handl_vm::Error::ResourceLimit).
    pub max_calls: Option<usize>,
}

impl Default for SandboxLimits {
    fn default() -> Self {
        SandboxLimits {
            max_depth: Some(256),
            max_calls: Some(100_000),
        }
    }
}

/// A sandboxed host with only pure library modules and resource limits.
///
/// Wraps [`CanonicalHost`] (built-in functions) and a [`ModuleHost`] pre-loaded
/// with every pure library module. Function lookup checks built-ins first,
/// then library modules. Use [`new_context`](Self::new_context) to obtain a
/// [`Context`] with the configured [`SandboxLimits`] applied.
#[derive(Clone)]
pub struct SandboxHost {
    default: CanonicalHost,
    modules: ModuleHost,
    limits: SandboxLimits,
}

impl SandboxHost {
    /// Creates a new sandbox host with the given limits and all pure
    /// library modules registered.
    pub fn new(limits: SandboxLimits) -> Self {
        let modules = ModuleHost::new()
            .with(MathModule)
            .with(StrModule)
            .with(ArrayModule)
            .with(SetModule)
            .with(BoolModule)
            .with(CmpModule)
            .with(ObjModule)
            .with(FnModule)
            .with(TypeModule)
            .with(CryptoModule)
            .with(DateTimeModule)
            .with(IntlModule::new())
            .with(KeyValueStoreModule::new(HashKeyValueStore::default()));

        SandboxHost {
            default: CanonicalHost::default(),
            modules,
            limits,
        }
    }

    /// Creates a [`Context`] with the sandbox limits applied.
    ///
    /// The VM will enforce `max_depth` and `max_calls` during evaluation,
    /// returning [`Error::ResourceLimit`](handl_vm::Error::ResourceLimit) if
    /// either limit is exceeded.
    pub fn new_context(&self) -> CanonicalContext {
        let mut ctx = CanonicalContext::new();
        if let Some(max) = self.limits.max_depth {
            ctx.set_max_depth(max);
        }
        if let Some(max) = self.limits.max_calls {
            ctx.set_max_calls(max);
        }
        ctx.set_eval(Box::new(CanonicalEval::new(
            self.clone(),
            self.limits.max_depth,
            self.limits.max_calls,
        )));
        ctx
    }

    /// Returns the configured limits.
    pub fn limits(&self) -> &SandboxLimits {
        &self.limits
    }

    /// Returns the inner [`ModuleHost`], allowing callers to register
    /// additional modules on top of the sandbox defaults.
    pub fn modules(&self) -> &ModuleHost {
        &self.modules
    }

    /// Consumes the host and returns the inner [`ModuleHost`],
    /// allowing callers to add more modules.
    pub fn into_modules(self) -> ModuleHost {
        self.modules
    }
}

impl Default for SandboxHost {
    fn default() -> Self {
        Self::new(SandboxLimits::default())
    }
}

impl Host<NonFunctionValue> for SandboxHost {
    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        self.default
            .function(name)
            .or_else(|| self.modules.function(name))
    }

    fn lazy_function(&self, name: &str) -> Option<&dyn LazyFunction<NonFunctionValue>> {
        self.default
            .lazy_function(name)
            .or_else(|| self.modules.lazy_function(name))
    }

    fn generic_function(&self, name: &str) -> Option<&dyn GenericFunction<NonFunctionValue>> {
        self.default
            .generic_function(name)
            .or_else(|| self.modules.generic_function(name))
    }

    fn is_reserved(&self, name: &str) -> bool {
        self.default.is_reserved(name) || self.modules.is_reserved(name)
    }
}

#[cfg(test)]
mod tests {
    use super::SandboxHost;
    use super::SandboxLimits;
    use handl_vm_values_json::Host;

    #[test]
    fn cmp_functions_are_available() {
        let host = SandboxHost::new(SandboxLimits::default());
        assert!(host.function("cmp.eq").is_some());
        assert!(host.function("cmp.compare").is_some());
        assert!(host.function("cmp.min").is_some());
        assert!(host.function("cmp.sort").is_some());
    }
}
