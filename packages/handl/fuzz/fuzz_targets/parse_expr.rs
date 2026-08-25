#![no_main]

use handl_typecheck_infer::parse_expr;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    if let Some(ast) = parse_expr(data, &handl_dialect_canonical::AC_DIALECT) {
        assert_valid_ast(&ast);
    }
});

fn assert_valid_ast(ast: &handl_typecheck_infer::ExprAst) {
    match ast {
        handl_typecheck_infer::ExprAst::Path(p) => {
            assert!(!p.is_empty(), "nested parse_expr produced empty Path");
        }
        handl_typecheck_infer::ExprAst::Call(name, args) => {
            assert!(!name.is_empty(), "nested parse_expr produced empty function name");
            for arg in args {
                assert_valid_ast(arg);
            }
        }
        handl_typecheck_infer::ExprAst::Literal(_) => {}
    }
}
