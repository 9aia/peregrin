use ac_ast::Expr;
use ac_host::DefaultHost;
use ac_host_resolver::ResolverHost;
use ac_module_resolver_core::ModuleResolver;
use ac_module_resolver_virtual::VirtualResolver;
use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Vm};
use serde_json::json;

// ── helpers ──────────────────────────────────────────────────────

fn make_resolver() -> VirtualResolver {
    let mut resolver = VirtualResolver::new();
    resolver.add(
        "config.yml",
        json!({
            "$": {
                "host": { "type": "string" },
                "port": { "type": "number" }
            },
            "host": "localhost",
            "port": 8080
        }),
    );
    resolver.add(
        "empty.yml",
        json!({
            "$": {}
        }),
    );
    resolver
}

fn val(v: impl Into<serde_json::Value>) -> NonFunctionValue {
    NonFunctionValue::new(v)
}

// ── import function tests ────────────────────────────────────────

#[test]
fn import_resolves_module_data() {
    let resolver = make_resolver();
    let host = ResolverHost::new(DefaultHost::default(), Box::new(resolver));
    let vm = Vm::new(host);
    let mut ctx = Context::new();

    let expr = Expr::call("import", vec![Expr::value(val("config.yml"))]);
    let result = vm.eval(&expr, &mut ctx).unwrap();

    let data = result.inner();
    assert_eq!(data["host"], "localhost");
    assert_eq!(data["port"], 8080);
}

#[test]
fn import_returns_cached_on_second_call() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct CountingResolver {
        modules: VirtualResolver,
        resolve_count: Arc<AtomicUsize>,
    }

    impl ModuleResolver for CountingResolver {
        fn resolve(
            &self,
            path: &str,
            base: Option<&std::path::Path>,
        ) -> Option<ac_module_resolver_core::ResolvedModule> {
            self.resolve_count.fetch_add(1, Ordering::SeqCst);
            self.modules.resolve(path, base)
        }
    }

    let resolve_count = Arc::new(AtomicUsize::new(0));

    let inner = make_resolver();
    let resolver = CountingResolver {
        modules: inner,
        resolve_count: resolve_count.clone(),
    };

    let host = ResolverHost::new(DefaultHost::default(), Box::new(resolver));
    let vm = Vm::new(host);
    let mut ctx = Context::new();

    let expr = Expr::call("import", vec![Expr::value(val("config.yml"))]);

    let result1 = vm.eval(&expr, &mut ctx).unwrap();
    let result2 = vm.eval(&expr, &mut ctx).unwrap();

    assert_eq!(result1.inner(), result2.inner());
    assert_eq!(resolve_count.load(Ordering::SeqCst), 1, "resolver should only be called once due to cache");
}

#[test]
fn import_errors_on_missing_module() {
    let resolver = VirtualResolver::new();
    let host = ResolverHost::new(DefaultHost::default(), Box::new(resolver));
    let vm = Vm::new(host);
    let mut ctx = Context::new();

    let expr = Expr::call("import", vec![Expr::value(val("nonexistent.yml"))]);
    let result = vm.eval(&expr, &mut ctx);

    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        Error::PrimitiveError(msg) => assert!(msg.contains("nonexistent.yml")),
        other => panic!("expected PrimitiveError, got {other:?}"),
    }
}

#[test]
fn import_errors_on_non_string_argument() {
    let resolver = make_resolver();
    let host = ResolverHost::new(DefaultHost::default(), Box::new(resolver));
    let vm = Vm::new(host);
    let mut ctx = Context::new();

    let expr = Expr::call("import", vec![Expr::value(val(42))]);
    let result = vm.eval(&expr, &mut ctx);

    assert!(result.is_err());
    match result.unwrap_err() {
        Error::TypeMismatch { expected, .. } => assert_eq!(expected, "string"),
        other => panic!("expected TypeMismatch, got {other:?}"),
    }
}

#[test]
fn import_errors_on_no_arguments() {
    let resolver = make_resolver();
    let host = ResolverHost::new(DefaultHost::default(), Box::new(resolver));
    let vm = Vm::new(host);
    let mut ctx = Context::new();

    let expr = Expr::call("import", vec![]);
    let result = vm.eval(&expr, &mut ctx);

    assert!(result.is_err());
    match result.unwrap_err() {
        Error::ArgumentCount { expected, got } => {
            assert_eq!(expected, 1);
            assert_eq!(got, 0);
        }
        other => panic!("expected ArgumentCount, got {other:?}"),
    }
}

// ── delegation tests ─────────────────────────────────────────────

#[test]
fn inner_host_functions_still_work() {
    let resolver = make_resolver();
    let host = ResolverHost::new(DefaultHost::default(), Box::new(resolver));
    let vm = Vm::new(host);
    let mut ctx = Context::new();

    let expr = Expr::call(
        "add",
        vec![Expr::value(val(2)), Expr::value(val(3))],
    );
    let result = vm.eval(&expr, &mut ctx).unwrap();

    assert_eq!(result.as_number(), Some(5.0));
}

#[test]
fn inner_host_functions_and_import_coexist() {
    let resolver = make_resolver();
    let host = ResolverHost::new(DefaultHost::default(), Box::new(resolver));
    let vm = Vm::new(host);
    let mut ctx = Context::new();

    let import_expr = Expr::call("import", vec![Expr::value(val("config.yml"))]);
    let imported = vm.eval(&import_expr, &mut ctx).unwrap();

    let port = imported.inner().get("port").unwrap();
    let add_expr = Expr::call(
        "add",
        vec![Expr::value(val(port.clone())), Expr::value(val(1))],
    );
    let result = vm.eval(&add_expr, &mut ctx).unwrap();

    assert_eq!(result.as_number(), Some(8081.0));
}

// ── with_base_path tests ─────────────────────────────────────────

#[test]
fn import_with_base_path() {
    let mut resolver = VirtualResolver::new();
    resolver.add(
        "base.yml",
        json!({
            "$": { "name": { "type": "string" } },
            "name": "from_base"
        }),
    );

    let host = ResolverHost::new(
        DefaultHost::default(),
        Box::new(resolver),
    )
    .with_base_path(std::path::PathBuf::from("/some/dir"));

    let vm = Vm::new(host);
    let mut ctx = Context::new();

    let expr = Expr::call("import", vec![Expr::value(val("base.yml"))]);
    let result = vm.eval(&expr, &mut ctx).unwrap();

    assert_eq!(result.inner()["name"], "from_base");
}

// ── empty module test ────────────────────────────────────────────

#[test]
fn import_empty_module() {
    let resolver = make_resolver();
    let host = ResolverHost::new(DefaultHost::default(), Box::new(resolver));
    let vm = Vm::new(host);
    let mut ctx = Context::new();

    let expr = Expr::call("import", vec![Expr::value(val("empty.yml"))]);
    let result = vm.eval(&expr, &mut ctx).unwrap();

    assert!(result.inner().is_object());
    assert!(result.inner().as_object().unwrap().is_empty());
}
