#![no_main]

use ac_typecheck_infer::parse_expr;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    if let Some(ast) = parse_expr(data) {
        assert_valid_ast(&ast);
    }
});

fn assert_valid_ast(ast: &ac_typecheck_infer::ExprAst) {
    match ast {
        ac_typecheck_infer::ExprAst::Path(p) => {
            assert!(!p.is_empty(), "nested parse_expr produced empty Path");
        }
        ac_typecheck_infer::ExprAst::Call(name, args) => {
            assert!(!name.is_empty(), "nested parse_expr produced empty function name");
            for arg in args {
                assert_valid_ast(arg);
            }
        }
        ac_typecheck_infer::ExprAst::Literal(_) => {}
    }
}
