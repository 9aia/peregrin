//! # handl-vm-server
//!
//! A [`Host`] for running handl expressions with effects enabled.
//!
//! Includes everything from [`handl-vm-config`] plus:
//!
//! - **OS stdlib** (`fs.*`, `env.*`, `path.*`, `time.*`, `cmd.*`, `process.*`, `tmp.*`, `socket.*`, `sig.*`)
//!   from [`handl-lib-os-std`].
//! - **HTTP server** (`http_server.*`) from [`handl-lib-http-server-std`].
//! - **HTTP client** (`http_client.*`) from [`handl-lib-http-client-std`].
//! - **Random** (`rand.*`) from [`handl-lib-rand`].
//!
//! This is the host you want when handl needs to open ports, read files,
//! spawn processes, or make HTTP requests.

use handl_lib_crypto::CryptoModule;
use handl_lib_datetime::DateTimeModule;
use handl_lib_http_client::HttpClientModule;
use handl_lib_http_client_std::StdHttpClient;
use handl_lib_http_server_std::std_http_server_module;
use handl_lib_intl::IntlModule;
use handl_lib_kv::KeyValueStoreModule;
use handl_lib_kv_hashmap::HashKeyValueStore;
use handl_lib_os::{
    CmdModule, EnvModule, FsModule, PathModule, ProcessModule, SigModule, SocketModule, TimeModule,
    TmpModule,
};
use handl_lib_os_std::{
    StdCmd, StdEnv, StdFs, StdPath, StdProcess, StdSig, StdSocket, StdTime, StdTmp,
};
use handl_lib_otlp_std::std_otlp_module;
use handl_lib_rand::RandModule;
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

/// Resource limits for server-side evaluation.
///
/// Mirrors [`handl_vm_config::SandboxLimits`] so the same depth/call
/// guards can be enforced even when effects are enabled.
#[derive(Clone, Debug)]
pub struct ServerLimits {
    /// Maximum scope depth (function call nesting).
    pub max_depth: Option<usize>,
    /// Maximum number of function calls during a single evaluation.
    pub max_calls: Option<usize>,
}

impl Default for ServerLimits {
    fn default() -> Self {
        ServerLimits {
            max_depth: Some(256),
            max_calls: Some(100_000),
        }
    }
}

/// A host with effects: sandbox modules plus HTTP, OS, and random.
///
/// Function lookup checks the built-in [`CanonicalHost`] first, then the
/// module host (pure libs, HTTP, OS, random, KV, etc.).
#[derive(Clone)]
pub struct ServerHost {
    default: CanonicalHost,
    modules: ModuleHost,
    limits: ServerLimits,
}

impl ServerHost {
    /// Creates a new server host with the given limits and all enabled modules.
    pub fn new(limits: ServerLimits) -> Self {
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
            .with(KeyValueStoreModule::new(HashKeyValueStore::default()))
            .with(RandModule)
            .with(FsModule::new(StdFs))
            .with(EnvModule::new(StdEnv))
            .with(PathModule::new(StdPath))
            .with(TimeModule::new(StdTime))
            .with(CmdModule::new(StdCmd))
            .with(ProcessModule::new(StdProcess::default()))
            .with(TmpModule::new(StdTmp))
            .with(SocketModule::new(StdSocket::default()))
            .with(SigModule::new(StdSig))
            .with(std_http_server_module())
            .with(HttpClientModule::new(StdHttpClient))
            .with(std_otlp_module());

        ServerHost {
            default: CanonicalHost::default(),
            modules,
            limits,
        }
    }

    /// Creates a [`Context`] with the server limits applied.
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
    pub fn limits(&self) -> &ServerLimits {
        &self.limits
    }

    /// Returns the inner [`ModuleHost`], allowing callers to register
    /// additional modules on top of the defaults.
    pub fn modules(&self) -> &ModuleHost {
        &self.modules
    }

    /// Consumes the host and returns the inner [`ModuleHost`].
    pub fn into_modules(self) -> ModuleHost {
        self.modules
    }
}

impl Default for ServerHost {
    fn default() -> Self {
        Self::new(ServerLimits::default())
    }
}

impl Host<NonFunctionValue> for ServerHost {
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
}

#[cfg(test)]
mod tests {
    use super::ServerHost;
    use super::ServerLimits;
    use handl_vm_values_json::Host;

    #[test]
    fn cmp_functions_are_available() {
        let host = ServerHost::new(ServerLimits::default());
        assert!(host.function("cmp.eq").is_some());
        assert!(host.function("cmp.compare").is_some());
        assert!(host.function("cmp.min").is_some());
        assert!(host.function("cmp.sort").is_some());
    }
}
