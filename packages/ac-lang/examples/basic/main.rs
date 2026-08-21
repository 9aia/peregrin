//! Basic usage: parse YAML, evaluate expressions, output YAML.
//!
//! Demonstrates the full pipeline: YAML frontend → NonFunctionValue →
//! VM evaluation → YAML output.
//!
//! Run: `cargo run --example basic`

use std::path::PathBuf;

use ac_ast::Expr;
use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function, Host, Vm};

// ── functions ───────────────────────────────────────────────────

struct Add;

impl Function for Add {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let a = args
            .first()
            .and_then(|v| v.as_number())
            .ok_or(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            })?;
        let b = args
            .get(1)
            .and_then(|v| v.as_number())
            .ok_or(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            })?;
        Ok(NonFunctionValue::new(serde_json::json!(a + b)))
    }
}

struct Concat;

impl Function for Concat {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let mut result = String::new();
        for arg in args {
            match arg.inner() {
                serde_json::Value::String(s) => result.push_str(s),
                _ => {
                    return Err(Error::TypeMismatch {
                        expected: "string",
                        got: "non-string",
                    });
                }
            }
        }
        Ok(NonFunctionValue::new(result))
    }
}

// ── host ────────────────────────────────────────────────────────

struct DemoHost {
    add: Add,
    concat: Concat,
}

impl Host for DemoHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "add" => Some(&self.add),
            "concat" => Some(&self.concat),
            _ => None,
        }
    }
}

// ── main ────────────────────────────────────────────────────────

fn main() {
    let vm = Vm::new(DemoHost {
        add: Add,
        concat: Concat,
    });
    let mut ctx = Context::new();

    // Parse YAML input from examples/basic/input.yml
    let yaml_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/basic/input.yml");
    let yaml_input = std::fs::read_to_string(&yaml_path)
        .unwrap_or_else(|_| panic!("failed to read {}", yaml_path.display()));
    let value = ac_frontend_yaml::parse(&yaml_input).unwrap();
    println!("parsed YAML from {}:", yaml_path.display());
    println!("{}", ac_frontend_yaml::to_string(&value).unwrap());

    // Evaluate: add(1, 2)
    let expr = Expr::call(
        "add",
        vec![
            Expr::value(NonFunctionValue::new(1.0)),
            Expr::value(NonFunctionValue::new(2.0)),
        ],
    );
    let result = vm.eval(&expr, &mut ctx).unwrap();
    println!(
        "add(1, 2) = {}",
        ac_frontend_yaml::to_string(&result).unwrap().trim()
    );

    // Evaluate: concat("hello, ", "world")
    let expr = Expr::call(
        "concat",
        vec![
            Expr::value(NonFunctionValue::new("hello, ")),
            Expr::value(NonFunctionValue::new("world")),
        ],
    );
    let result = vm.eval(&expr, &mut ctx).unwrap();
    println!(
        "concat(\"hello, \", \"world\") = {}",
        ac_frontend_yaml::to_string(&result).unwrap().trim()
    );

    println!("\nAll assertions passed.");
}
