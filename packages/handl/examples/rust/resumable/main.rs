//! Resumable evaluation: cross-machine continuation serialization.
//!
//! This example demonstrates how to pause evaluation on one VM,
//! serialize the continuation to JSON, and resume it on a different
//! VM with a different host — without re-evaluating from scratch.
//!
//! Run: `cargo run --example resumable`

use handl_ast_canonical::Expr;
use handl_vm::NonFunctionValue;
use handl_vm_canonical::{
    CanonicalContext, Context, Error, EvalResult, Function, Host, Operation, OperationSig, Vm,
};

// ── functions ─────────────────────────────────────────────────────

struct Add;
impl Function for Add {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let sum: f64 = args.iter().filter_map(|v| v.as_number()).sum();
        Ok(NonFunctionValue::new(sum))
    }
}

// ── operations ────────────────────────────────────────────────────

struct FetchOp;
impl Operation for FetchOp {
    fn signature(&self) -> OperationSig {
        OperationSig {
            name: "fetch".to_string(),
            params: vec![("url".to_string(), serde_json::json!({"type": "string"}))],
            return_schema: Some(serde_json::json!({"type": "string"})),
        }
    }
    fn perform(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        Err(Error::Operation {
            name: "fetch".to_string(),
            args: args.to_vec(),
        })
    }
}

/// A caching fetch — returns a cached value for known URLs.
struct CachingFetch;
impl Operation for CachingFetch {
    fn signature(&self) -> OperationSig {
        OperationSig {
            name: "fetch".to_string(),
            params: vec![],
            return_schema: None,
        }
    }
    fn perform(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        Ok(NonFunctionValue::new(42.0))
    }
}

// ── hosts ─────────────────────────────────────────────────────────

struct ServerHost {
    add: Add,
    fetch: FetchOp,
}
impl Host for ServerHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "add" => Some(&self.add),
            _ => None,
        }
    }
    fn operation(&self, name: &str) -> Option<&dyn Operation> {
        match name {
            "fetch" => Some(&self.fetch),
            _ => None,
        }
    }
}
impl Default for ServerHost {
    fn default() -> Self {
        ServerHost {
            add: Add,
            fetch: FetchOp,
        }
    }
}

struct ClientHost {
    add: Add,
    fetch: CachingFetch,
}
impl Host for ClientHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "add" => Some(&self.add),
            _ => None,
        }
    }
    fn operation(&self, name: &str) -> Option<&dyn Operation> {
        match name {
            "fetch" => Some(&self.fetch),
            _ => None,
        }
    }
}
impl Default for ClientHost {
    fn default() -> Self {
        ClientHost {
            add: Add,
            fetch: CachingFetch,
        }
    }
}

// ── main ──────────────────────────────────────────────────────────

fn main() {
    // Expression: add(fetch("data"), 10)
    let expr = Expr::call(
        "add",
        vec![
            Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
            Expr::value(NonFunctionValue::new(10)),
        ],
    );

    println!("=== Resumable Evaluation Demo ===");
    println!("Expression: add(fetch(\"data\"), 10)");

    // ── Server: start evaluation ────────────────────────────────
    let server_vm = Vm::new(ServerHost::default());
    let mut ctx = CanonicalContext::new();
    let result = server_vm.eval_resumable(&expr, &mut ctx, None);

    let (yielded_value, continuation) = match result {
        EvalResult::Yielded {
            value,
            continuation,
        } => (value, continuation),
        other => panic!("expected Yielded, got {other:?}"),
    };
    println!("\n[Server] fetch operation signaled");
    println!("[Server] Yielded value: {:?}", yielded_value);
    println!(
        "[Server] Continuation has {} frames",
        continuation.frames.len()
    );

    // Serialize continuation to JSON
    let cont_json = continuation.to_json();
    let cont_str = serde_json::to_string_pretty(&cont_json).unwrap();
    println!(
        "[Server] Serialized continuation ({} bytes)",
        cont_str.len()
    );

    // ── Client: resume evaluation ───────────────────────────────
    let restored = handl_vm_canonical::Continuation::from_json(&cont_json).unwrap();
    let client_vm = Vm::new(ClientHost::default());
    let result = client_vm.eval_resumable(&expr, &mut ctx, Some(restored));

    match result {
        EvalResult::Done(value) => {
            println!("\n[Client] Resumed with caching fetch (returns 42)");
            println!(
                "[Client] Result: add(42, 10) = {}",
                value.as_number().unwrap()
            );
        }
        other => panic!("expected Done, got {other:?}"),
    }

    println!("\nResumable evaluation demo passed.");
}
