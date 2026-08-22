//! Minimal demo: functions as host sugar.
//! Run: `cargo run -p experiment-host-sugar`

use std::cell::Cell;
use std::collections::HashMap;

use ac_compiler::VmResolver;
use ac_dialect_canonical::AC_DIALECT;
use ac_fields::{DEFAULT_NAMESPACE, build_doc_namespace, eval_expression};
use ac_host::DefaultHost;
use ac_module_resolver_file::FileResolver;
use ac_module_resolver_memory::MemoryModuleCache;
use ac_runtime::NonFunctionValue;

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
    let resolver = VmResolver {
        host: DefaultHost::default(),
        namespaces: ns.clone(),
        depth: Cell::new(0),
        base_path: None,
        import_cache: std::cell::RefCell::new(HashMap::new()),
        resolver: Some(&fr),
        type_env: None,
        function_table: None,
    };

    println!("\n--- expression ---\n");

    // double(5) → host sugar finds doc.functions.double (has body) → apply → mul(5, 2) = 10
    let r = eval_expression("double(n)", &ns, &resolver, &AC_DIALECT).unwrap();
    println!("double(n)");
    println!("  → doc.functions.double has 'body' → dispatch via apply");
    println!("  → apply binds x=5, evaluates mul(x, 2)");
    println!("  = {r}");
}
