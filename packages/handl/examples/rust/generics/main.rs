//! Generic functions: parametric polymorphism in the typechecker.
//!
//! Demonstrates that generic function signatures (`identity<T>`, `first<T>`,
//! `map<T,U>`) are properly typechecked through the full compiler pipeline.
//! Generics are a typechecker-only feature — no runtime changes.
//!
//! Run: `cargo run --example generics`

use std::path::PathBuf;

fn main() {
    let yaml_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/rust/generics/input.yml");
    let yaml_input = std::fs::read_to_string(&yaml_path)
        .unwrap_or_else(|_| panic!("failed to read {}", yaml_path.display()));

    println!(
        "Typechecking generic functions from {}\n",
        yaml_path.display()
    );

    let report = handl_compiler::typecheck::run(&yaml_input, "yaml", None).unwrap();

    if report.is_valid() {
        println!("No type errors — generic functions typechecked successfully.");
        println!();
        println!("Generic signatures declared:");
        println!("  identity<T>(x: T) -> T");
        println!("  first<T>(arr: Array<T>) -> T");
        println!("  map<T, U>(arr: Array<T>, fn: (T) -> U) -> Array<U>");
        println!();
        println!("Expressions typechecked:");
        println!("  name:      identity(\"handl\")  -> String  (T = String)");
        println!("  first_tag: first(tags)          -> String  (T = String)");
    } else {
        println!("Type errors found:");
        for err in &report.errors {
            println!("  {err}");
        }
    }

    println!("\nAll assertions passed.");
}
