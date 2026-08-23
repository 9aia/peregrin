//! Minimal demo: functions as host sugar.
//! Run: `cargo run -p experiment-host-sugar`

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ac_compiler_canonical::{CompilerHost, Import};
use ac_dialect_canonical::AC_DIALECT;
use ac_fields::{DEFAULT_NAMESPACE, build_doc_namespace, eval_expression};
use ac_module_resolver_file::FileResolver;
use ac_module_resolver_memory::MemoryModuleCache;
use ac_runtime::NonFunctionValue;
use ac_runtime_example_1::DefaultHost;

fn main() {
    let yaml = r#"
$:
  n:
    type: number
  functions:
    type: object
  doubled:
    type: number
    expr: "double(n)"

functions:
  double:
    params:
      x: { type: number }
    body: "mul(x, 2)"

n: 5
"#;
    println!("--- input YAML ---");
    println!("{yaml}");

    // `functions` is just a regular field. `double` has a `body` → it's a function.
    let doc = NonFunctionValue::new(serde_json::json!({
        "n": 5,
        "functions": {
            "double": { "params": { "x": { "type": "number" } }, "body": "mul(x, 2)" }
        }
    }));

    println!("--- doc namespace ---");
    println!("{}", serde_json::to_string_pretty(&doc.inner()).unwrap());

    let mut ns = HashMap::new();
    ns.insert(DEFAULT_NAMESPACE.to_string(), build_doc_namespace(&doc));

    let fr = FileResolver::with_cache(MemoryModuleCache::new());
    let import = Import::new(
        None,
        Rc::new(Box::new(fr) as Box<dyn ac_module_resolver_core::ModuleResolver>),
        Rc::new(RefCell::new(HashMap::new())),
    );
    let host = CompilerHost::new(DefaultHost::default(), import, &ns);

    println!("\n--- expression ---\n");

    // double(5) → host sugar finds doc.functions.double (has body) → apply → mul(5, 2) = 10
    let r = eval_expression("double(n)", &ns, &host, &AC_DIALECT).unwrap();
    println!("double(n)");
    println!("  → doc.functions.double has 'body' → dispatch via apply");
    println!("  → apply binds x=5, evaluates mul(x, 2)");
    println!("  = {r}");
}
