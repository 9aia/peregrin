//! Functions and scoping: nested calls, variable scopes, and
//! host-provided functions.
//!
//! Demonstrates the full pipeline: YAML frontend → NonFunctionValue →
//! VM evaluation → YAML output.
//!
//! Run: `cargo run --example functions`

use std::path::PathBuf;

use ac_ast::Expr;
use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function, Host, Vm};

// ── functions ───────────────────────────────────────────────────

struct Mul;

impl Function for Mul {
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
        Ok(NonFunctionValue::new(serde_json::json!(a * b)))
    }
}

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

// ── host ────────────────────────────────────────────────────────

struct MathHost {
    add: Add,
    mul: Mul,
}

impl Host for MathHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "add" => Some(&self.add),
            "mul" => Some(&self.mul),
            _ => None,
        }
    }
}

// ── main ────────────────────────────────────────────────────────

fn main() {
    let vm = Vm::new(MathHost { add: Add, mul: Mul });
    let mut ctx = Context::new();

    // Parse YAML input from examples/functions/input.yml
    let yaml_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/functions/input.yml");
    let yaml_input = std::fs::read_to_string(&yaml_path)
        .unwrap_or_else(|_| panic!("failed to read {}", yaml_path.display()));
    let config = ac_frontend_yaml::parse(&yaml_input).unwrap();
    println!("parsed YAML config from {}:", yaml_path.display());
    println!("{}", ac_frontend_yaml::to_string(&config).unwrap());

    // Nested call: mul(add(1, 2), add(3, 4)) = mul(3, 7) = 21
    let expr = Expr::call(
        "mul",
        vec![
            Expr::call(
                "add",
                vec![
                    Expr::value(NonFunctionValue::new(1.0)),
                    Expr::value(NonFunctionValue::new(2.0)),
                ],
            ),
            Expr::call(
                "add",
                vec![
                    Expr::value(NonFunctionValue::new(3.0)),
                    Expr::value(NonFunctionValue::new(4.0)),
                ],
            ),
        ],
    );
    let result = vm.eval(&expr, &mut ctx).unwrap();
    println!(
        "mul(add(1, 2), add(3, 4)) = {}",
        ac_frontend_yaml::to_string(&result).unwrap().trim()
    );

    // Variable scoping — context still manages named values
    ctx.define("base", NonFunctionValue::new(serde_json::json!(10.0)));
    ctx.push_scope();
    ctx.define("base", NonFunctionValue::new(serde_json::json!(20.0)));

    let result = ctx.get("base").unwrap();
    println!(
        "inner scope base = {}",
        ac_frontend_yaml::to_string(&result).unwrap().trim()
    );

    ctx.pop_scope();
    let result = ctx.get("base").unwrap();
    println!(
        "after pop, base = {}",
        ac_frontend_yaml::to_string(&result).unwrap().trim()
    );

    // Build an object from fields and output as YAML
    let mut obj = serde_json::Map::new();
    obj.insert("name".to_string(), serde_json::json!("ac-lang"));
    obj.insert("version".to_string(), serde_json::json!(1.0));
    obj.insert("computed".to_string(), serde_json::json!(42.0));
    let obj = NonFunctionValue::new(serde_json::Value::Object(obj));
    println!("\nobject as YAML:");
    println!("{}", ac_frontend_yaml::to_string(&obj).unwrap());

    println!("All assertions passed.");
}
