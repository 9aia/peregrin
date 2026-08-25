use handl_lexer_canonical::CanonicalLexer;
use handl_parser::Parser;
use handl_parser_canonical::CanonicalParser;
use handl_vm::{Error, Eval, Host};
use handl_vm_values_json::{CanonicalContext, NonFunctionValue};

use crate::CanonicalVm;

/// A synchronous [`Eval`] callback for the canonical handl runtime.
///
/// Parses the expression string with the canonical parser, binds the
/// provided variables, and evaluates the tree with a fresh VM using
/// the supplied host. This enables higher-order functions like
/// `fn.tap`, `fn.apply`, and `fn.pipe` in the canonical VM.
#[derive(Clone)]
pub struct CanonicalEval<H> {
    host: H,
    max_depth: Option<usize>,
    max_calls: Option<usize>,
}

impl<H> CanonicalEval<H> {
    /// Creates a new canonical eval callback.
    pub fn new(host: H, max_depth: Option<usize>, max_calls: Option<usize>) -> Self {
        CanonicalEval {
            host,
            max_depth,
            max_calls,
        }
    }
}

impl<H: Host<NonFunctionValue> + Clone + 'static> Eval<NonFunctionValue> for CanonicalEval<H> {
    fn eval_expr(
        &self,
        expr: &str,
        vars: &[(String, NonFunctionValue)],
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let tree = CanonicalParser
            .parse_text(expr, &CanonicalLexer::default())
            .map_err(|e| Error::PrimitiveError(e.to_string()))?;

        let mut ctx = CanonicalContext::new();
        for (name, value) in vars {
            ctx.define(name, value.clone());
        }
        if let Some(max) = self.max_depth {
            ctx.set_max_depth(max);
        }
        if let Some(max) = self.max_calls {
            ctx.set_max_calls(max);
        }
        ctx.set_eval(Box::new(self.clone()));

        let vm = CanonicalVm::new(self.host.clone());
        vm.eval(&tree, &mut ctx)
    }
}
