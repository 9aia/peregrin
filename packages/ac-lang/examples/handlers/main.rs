//! Handlers: algebraic effect handling in ac-lang.
//!
//! This example demonstrates how to intercept operations with handler
//! expressions. An operation signals (yields) instead of completing,
//! and a handler can intercept it, inspect the arguments, and either
//! resume the continuation or return a different value.
//!
//! Run: `cargo run --example handlers`

use ac_ast::{Expr, HandlerClause};
use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function, Host, Operation, OperationSig, Vm};

// ── functions ─────────────────────────────────────────────────────

struct Add;
impl Function for Add {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let sum: f64 = args.iter().filter_map(|v| v.as_number()).sum();
        Ok(NonFunctionValue::new(sum))
    }
}

struct Concat;
impl Function for Concat {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let s: String = args
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<_>>()
            .join("");
        Ok(NonFunctionValue::new(s))
    }
}

// ── operations ────────────────────────────────────────────────────

/// A fetch operation that always signals (yields to handler).
struct FetchOp;
impl Operation for FetchOp {
    fn signature(&self) -> OperationSig {
        OperationSig {
            name: "fetch".to_string(),
            params: vec![("url".to_string(), serde_json::json!({"type": "string"}))],
            return_schema: Some(serde_json::json!({"type": "string"})),
        }
    }
    fn perform(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        Err(Error::Operation {
            name: "fetch".to_string(),
            args: args.to_vec(),
        })
    }
}

// ── host ──────────────────────────────────────────────────────────

struct DemoHost {
    add: Add,
    concat: Concat,
    fetch: FetchOp,
}

impl Host for DemoHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "add" => Some(&self.add),
            "concat" => Some(&self.concat),
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

impl Default for DemoHost {
    fn default() -> Self {
        DemoHost {
            add: Add,
            concat: Concat,
            fetch: FetchOp,
        }
    }
}

// ── main ──────────────────────────────────────────────────────────

fn main() {
    let vm = Vm::new(DemoHost::default());
    let mut ctx = Context::new();

    // Example 1: Handler intercepts fetch and returns a constant
    println!("=== Example 1: Handler returns constant ===");
    let expr = Expr::handle(
        Expr::call(
            "fetch",
            vec![Expr::value(NonFunctionValue::new(
                "https://api.example.com/data",
            ))],
        ),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec!["url".to_string()],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new("intercepted-data")),
        }],
    );
    let result = vm.eval(&expr, &mut ctx).unwrap();
    println!(
        "handle(fetch(\"https://api.example.com/data\")) = {}",
        result.as_str().unwrap()
    );

    // Example 2: Handler resumes the continuation with a value
    println!("\n=== Example 2: Handler resumes continuation ===");
    let expr = Expr::handle(
        Expr::call(
            "add",
            vec![
                Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
                Expr::value(NonFunctionValue::new(10)),
            ],
        ),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec!["url".to_string()],
            continuation_name: "k".to_string(),
            body: Expr::call("k", vec![Expr::value(NonFunctionValue::new(42.0))]),
        }],
    );
    let result = vm.eval(&expr, &mut ctx).unwrap();
    println!(
        "handle(add(fetch(\"data\"), 10)) with k(42) = {}",
        result.as_number().unwrap()
    );

    // Example 3: Handler resumes with a computed value
    println!("\n=== Example 3: Handler resumes with computed value ===");
    let expr = Expr::handle(
        Expr::call(
            "concat",
            vec![
                Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("name"))]),
                Expr::value(NonFunctionValue::new("!")),
            ],
        ),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec!["url".to_string()],
            continuation_name: "k".to_string(),
            body: Expr::call("k", vec![Expr::value(NonFunctionValue::new("World"))]),
        }],
    );
    let result = vm.eval(&expr, &mut ctx).unwrap();
    println!(
        "handle(concat(fetch(\"name\"), \"!\")) with k(\"World\") = {}",
        result.as_str().unwrap()
    );

    // Example 4: Handler does NOT resume — discards the continuation
    println!("\n=== Example 4: Handler discards continuation ===");
    let expr = Expr::handle(
        Expr::call(
            "add",
            vec![
                Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
                Expr::value(NonFunctionValue::new(100)),
            ],
        ),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec!["url".to_string()],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new(999.0)),
        }],
    );
    let result = vm.eval(&expr, &mut ctx).unwrap();
    println!(
        "handle(add(fetch(\"data\"), 100)) with constant 999 = {}",
        result.as_number().unwrap()
    );

    println!("\nAll handler examples passed.");
}
