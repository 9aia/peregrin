use ac_runtime_values::NonFunctionValue;
use std::sync::Arc;

use ac_runtime::{Context, Function};
use ac_runtime_modules::Module;
use ac_runtime_values::Error;

use crate::error::OsError;

/// Abstract signal service.
///
/// Implementations:
/// - `ac-lib-os-std` — backed by platform signal APIs
/// - [`sig_stub::StubSig`] — canned responses for testing
pub trait Sig: Send + Sync {
    fn send(&self, process_id: i64, signal: &str) -> Result<(), OsError>;
    fn list(&self) -> Result<Vec<String>, OsError>;
}

/// Module adapter for [`Sig`] — exposes `sig.*` functions.
pub struct SigModule {
    send: SendFn,
    list: ListFn,
}

impl SigModule {
    pub fn new(sig: impl Sig + 'static) -> Self {
        let sig = Arc::new(sig);
        SigModule {
            send: SendFn { sig: sig.clone() },
            list: ListFn { sig },
        }
    }
}

impl Module for SigModule {
    fn name(&self) -> &str {
        "sig"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "send" => Some(&self.send),
            "list" => Some(&self.list),
            _ => None,
        }
    }
}

struct SendFn {
    sig: Arc<dyn Sig>,
}

impl Function<NonFunctionValue> for SendFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let process_id = args[0].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })? as i64;
        let signal = args[1].as_str().ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })?;
        self.sig.send(process_id, signal)?;
        Ok(NonFunctionValue::default())
    }
}

struct ListFn {
    sig: Arc<dyn Sig>,
}

impl Function<NonFunctionValue> for ListFn {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let signals = self.sig.list()?;
        let arr: Vec<serde_json::Value> =
            signals.into_iter().map(serde_json::Value::String).collect();
        Ok(NonFunctionValue::new(serde_json::Value::Array(arr)))
    }
}
