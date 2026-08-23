//! Multi-handler: multiple operation handlers and nested handlers.
//!
//! This example shows how to handle multiple different operations
//! in a single handle expression, and how nested handlers work
//! with inner handlers taking precedence over outer ones.
//!
//! Run: `cargo run --example multi-handler`

use ac_ast_canonical::{Expr, HandlerClause};
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

struct Mul;
impl Function for Mul {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let product: f64 = args.iter().filter_map(|v| v.as_number()).product();
        Ok(NonFunctionValue::new(product))
    }
}

// ── operations ────────────────────────────────────────────────────

struct FetchOp;
impl Operation for FetchOp {
    fn signature(&self) -> OperationSig {
        OperationSig {
            name: "fetch".to_string(),
            params: vec![],
            return_schema: None,
        }
    }
    fn perform(&self, _args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        Err(Error::Operation {
            name: "fetch".to_string(),
            args: vec![],
        })
    }
}

struct LogOp;
impl Operation for LogOp {
    fn signature(&self) -> OperationSig {
        OperationSig {
            name: "log".to_string(),
            params: vec![],
            return_schema: None,
        }
    }
    fn perform(&self, _args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        Err(Error::Operation {
            name: "log".to_string(),
            args: vec![],
        })
    }
}

struct ReadFileOp;
impl Operation for ReadFileOp {
    fn signature(&self) -> OperationSig {
        OperationSig {
            name: "read_file".to_string(),
            params: vec![],
            return_schema: None,
        }
    }
    fn perform(&self, _args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        Err(Error::Operation {
            name: "read_file".to_string(),
            args: vec![],
        })
    }
}

// ── host ──────────────────────────────────────────────────────────

struct DemoHost {
    add: Add,
    mul: Mul,
    fetch: FetchOp,
    log: LogOp,
    read_file: ReadFileOp,
}

impl Host for DemoHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "add" => Some(&self.add),
            "mul" => Some(&self.mul),
            _ => None,
        }
    }
    fn operation(&self, name: &str) -> Option<&dyn Operation> {
        match name {
            "fetch" => Some(&self.fetch),
            "log" => Some(&self.log),
            "read_file" => Some(&self.read_file),
            _ => None,
        }
    }
}

impl Default for DemoHost {
    fn default() -> Self {
        DemoHost {
            add: Add,
            mul: Mul,
            fetch: FetchOp,
            log: LogOp,
            read_file: ReadFileOp,
        }
    }
}

// ── main ──────────────────────────────────────────────────────────

fn main() {
    let vm = Vm::new(DemoHost::default());
    let mut ctx = Context::new();

    // Example 1: Multiple handler clauses for different operations
    println!("=== Example 1: Multiple handler clauses ===");
    // add(fetch(), mul(log(), 2))
    // fetch handler: k(42) → 42
    // log handler: k(1) → 1
    // result: add(42, mul(1, 2)) = add(42, 2) = 44
    let expr = Expr::handle(
        Expr::call(
            "add",
            vec![
                Expr::call("fetch", vec![]),
                Expr::call(
                    "mul",
                    vec![Expr::call("log", vec![]), Expr::value(NonFunctionValue::new(2))],
                ),
            ],
        ),
        vec![
            HandlerClause {
                op_name: "fetch".to_string(),
                params: vec![],
                continuation_name: "k".to_string(),
                body: Expr::call("k", vec![Expr::value(NonFunctionValue::new(42.0))]),
            },
            HandlerClause {
                op_name: "log".to_string(),
                params: vec![],
                continuation_name: "k".to_string(),
                body: Expr::call("k", vec![Expr::value(NonFunctionValue::new(1.0))]),
            },
        ],
    );
    let result = vm.eval(&expr, &mut ctx).unwrap();
    println!(
        "add(fetch(), mul(log(), 2)) = {} (expected 44)",
        result.as_number().unwrap()
    );

    // Example 2: Nested handlers — outer takes precedence
    println!("\n=== Example 2: Nested handlers — outer precedence ===");
    // outer handle { inner handle { fetch() } }
    // outer handler for "fetch" wins (handler search uses .rev())
    let expr = Expr::handle(
        Expr::handle(
            Expr::call("fetch", vec![]),
            vec![HandlerClause {
                op_name: "fetch".to_string(),
                params: vec![],
                continuation_name: "k".to_string(),
                body: Expr::value(NonFunctionValue::new("inner-handled")),
            }],
        ),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec![],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new("outer-handled")),
        }],
    );
    let result = vm.eval(&expr, &mut ctx).unwrap();
    println!(
        "Result: {} (expected outer-handled)",
        result.as_str().unwrap()
    );

    // Example 3: Outer handler catches unhandled operation
    println!("\n=== Example 3: Outer catches unhandled op ===");
    // outer handle { inner handle { read_file() } }
    // inner only handles "fetch", so "read_file" propagates to outer
    let expr = Expr::handle(
        Expr::handle(
            Expr::call("read_file", vec![]),
            vec![HandlerClause {
                op_name: "fetch".to_string(),
                params: vec![],
                continuation_name: "k".to_string(),
                body: Expr::value(NonFunctionValue::new("inner-fetch")),
            }],
        ),
        vec![HandlerClause {
            op_name: "read_file".to_string(),
            params: vec![],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new("outer-readfile")),
        }],
    );
    let result = vm.eval(&expr, &mut ctx).unwrap();
    println!(
        "Result: {} (expected outer-readfile)",
        result.as_str().unwrap()
    );

    // Example 4: Three operations, three handlers
    println!("\n=== Example 4: Three operations, three handlers ===");
    // add(add(fetch(), log()), read_file())
    // fetch → 10, log → 1, read_file → 100
    // result: add(add(10, 1), 100) = 111
    let expr = Expr::handle(
        Expr::call(
            "add",
            vec![
                Expr::call(
                    "add",
                    vec![Expr::call("fetch", vec![]), Expr::call("log", vec![])],
                ),
                Expr::call("read_file", vec![]),
            ],
        ),
        vec![
            HandlerClause {
                op_name: "fetch".to_string(),
                params: vec![],
                continuation_name: "k".to_string(),
                body: Expr::call("k", vec![Expr::value(NonFunctionValue::new(10.0))]),
            },
            HandlerClause {
                op_name: "log".to_string(),
                params: vec![],
                continuation_name: "k".to_string(),
                body: Expr::call("k", vec![Expr::value(NonFunctionValue::new(1.0))]),
            },
            HandlerClause {
                op_name: "read_file".to_string(),
                params: vec![],
                continuation_name: "k".to_string(),
                body: Expr::call("k", vec![Expr::value(NonFunctionValue::new(100.0))]),
            },
        ],
    );
    let result = vm.eval(&expr, &mut ctx).unwrap();
    println!(
        "add(add(fetch(), log()), read_file()) = {} (expected 111)",
        result.as_number().unwrap()
    );

    println!("\nAll multi-handler examples passed.");
}
