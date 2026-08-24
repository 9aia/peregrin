use ac_runtime_modules::Module;
use ac_runtime_values::CanonicalContext;
use ac_runtime_values::NonFunctionValue;

use super::fs::FsModule;
use super::fs_stub::StubFs;

fn call(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> ac_runtime_values::Error {
    let func = module.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).unwrap_err()
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

#[test]
fn module_name() {
    let module = FsModule::new(StubFs::default());
    assert_eq!(module.name(), "fs");
}

#[test]
fn read_file() {
    let fs = StubFs::with_file("hello.txt", "hello world");
    let module = FsModule::new(fs);
    assert_eq!(
        call(&module, "read", &[s("hello.txt")]).as_str(),
        Some("hello world")
    );
}

#[test]
fn read_file_not_found() {
    let module = FsModule::new(StubFs::default());
    let err = call_err(&module, "read", &[s("missing.txt")]);
    match err {
        ac_runtime_values::Error::PrimitiveError(msg) => assert!(msg.contains("not found")),
        other => panic!("expected PrimitiveError, got {other:?}"),
    }
}

#[test]
fn write_file() {
    let module = FsModule::new(StubFs::default());
    call(&module, "write", &[s("out.txt"), s("data")]);
    assert_eq!(
        call(&module, "read", &[s("out.txt")]).as_str(),
        Some("data")
    );
}

#[test]
fn exists_true() {
    let fs = StubFs::with_file("a.txt", "x");
    let module = FsModule::new(fs);
    assert_eq!(call(&module, "exists", &[s("a.txt")]).as_bool(), Some(true));
}

#[test]
fn exists_false() {
    let module = FsModule::new(StubFs::default());
    assert_eq!(call(&module, "exists", &[s("nope")]).as_bool(), Some(false));
}

#[test]
fn remove_file() {
    let fs = StubFs::with_file("temp.txt", "x");
    let module = FsModule::new(fs);
    call(&module, "remove", &[s("temp.txt")]);
    assert_eq!(
        call(&module, "exists", &[s("temp.txt")]).as_bool(),
        Some(false)
    );
}

#[test]
fn list_dir() {
    let fs = StubFs::default();
    {
        let mut files = fs.files.lock().unwrap();
        files.insert("dir/a.txt".into(), "1".into());
        files.insert("dir/b.txt".into(), "2".into());
        files.insert("other/c.txt".into(), "3".into());
    }
    let module = FsModule::new(fs);
    let result = call(&module, "list", &[s("dir/")]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);
}

#[test]
fn unknown_function() {
    let module = FsModule::new(StubFs::default());
    assert!(module.function("nope").is_none());
}

#[test]
fn argument_count() {
    let module = FsModule::new(StubFs::default());
    let err = call_err(&module, "read", &[]);
    assert_eq!(
        err,
        ac_runtime_values::Error::ArgumentCount {
            expected: 1,
            got: 0
        }
    );
}
