use ac_ast_canonical::Expr;
use ac_frontend::frontend_for;
use ac_lexer_canonical::CanonicalLexer;
use ac_parser::Parser;
use ac_parser_canonical::CanonicalParser;
use ac_runtime_canonical::Vm;
use ac_runtime_sandbox::{SandboxHost, SandboxLimits};
use ac_runtime_server::{ServerHost, ServerLimits};
use ac_runtime_values::NonFunctionValue;

use crate::{
    Config, DriverError, DriverPipeline, ParseStage, ParsedDoc, Session, Source, Stage,
    default_cli_config,
};

/// Which VM host to use for evaluation.
#[derive(Clone, Copy, Debug)]
pub enum VmMode {
    /// Pure, sandboxed evaluation (default).
    Sandbox,
    /// Effects-enabled evaluation (files, HTTP, OS, ...).
    Server,
}

/// Eval phase: `ParsedDoc → NonFunctionValue`.
///
/// Runs the canonical AC VM over the parsed document, treating the
/// document root as the expression to evaluate.
pub struct EvalStage;

impl Stage<Config> for EvalStage {
    type Input = ParsedDoc;
    type Output = NonFunctionValue;

    fn name(&self) -> &'static str {
        "eval"
    }

    fn run(
        &mut self,
        doc: ParsedDoc,
        session: &mut Session<Config>,
    ) -> Result<NonFunctionValue, DriverError> {
        let mode = session
            .config()
            .extension::<VmMode>()
            .copied()
            .unwrap_or(VmMode::Sandbox);

        let (expr, defines) = if doc.format == "ac" {
            let src = doc.value.as_str().ok_or_else(|| {
                DriverError::new(".ac input did not parse as a string".to_string())
            })?;
            let expr = CanonicalParser
                .parse_text(src, &CanonicalLexer::default())
                .map_err(|e| DriverError::new(e.to_string()))?;
            (expr, Vec::new())
        } else {
            let mut defines = Vec::new();
            if let Some(obj) = doc.value.as_object() {
                for (k, v) in obj {
                    defines.push((k.clone(), NonFunctionValue::new(v.clone())));
                }
            }
            (Expr::value(doc.value), defines)
        };

        match mode {
            VmMode::Server => {
                let host = ServerHost::new(ServerLimits::default());
                let mut ctx = host.new_context();
                for (k, v) in &defines {
                    ctx.define(k, v.clone());
                }
                let vm = Vm::new(host);
                vm.eval(&expr, &mut ctx)
                    .map_err(|e| DriverError::new(e.to_string()))
            }
            VmMode::Sandbox => {
                let host = SandboxHost::new(SandboxLimits::default());
                let mut ctx = host.new_context();
                for (k, v) in &defines {
                    ctx.define(k, v.clone());
                }
                let vm = Vm::new(host);
                vm.eval(&expr, &mut ctx)
                    .map_err(|e| DriverError::new(e.to_string()))
            }
        }
    }
}

/// Evaluate an AC document through the parse → VM pipeline.
pub fn run(input: &str, format: &str, output_format: &str, vm: &str) -> Result<String, String> {
    let vm_mode = if vm == "server" {
        VmMode::Server
    } else {
        VmMode::Sandbox
    };
    let source = Source::new(input, format);
    let mut pipeline = DriverPipeline::new(ParseStage).then(EvalStage);
    let mut session = Session::new(default_cli_config().with_extension(vm_mode));
    let value = pipeline
        .run(source, &mut session)
        .map_err(|e| e.to_string())?;
    let output_frontend = frontend_for(output_format)?;
    output_frontend.serialize(&value)
}
