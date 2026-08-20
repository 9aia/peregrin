//! # ac-engine-pass-typecheck
//!
//! Pipeline pass: typecheck all AC keys and cross-module imports.

use ac_engine_core::{Compilation, Config, Pass};

/// Typecheck all AC keys and cross-module imports.
///
/// Populates `compilation.type_errors` with any type mismatches found.
pub struct TypecheckPass;

impl Pass for TypecheckPass {
    fn should_run(&self, compilation: &Compilation, _config: &Config) -> bool {
        compilation.extracted.is_some()
    }

    fn run(&self, compilation: &mut Compilation, config: &Config) -> Result<(), String> {
        let extracted = compilation
            .extracted
            .as_ref()
            .ok_or("TypecheckPass: no extracted fields — run ExtractPass first")?;

        let interpreter = config.interpreter.as_ref();
        let data = extracted.data.inner().clone();

        let mut env = ac_typecheck_core::TypeEnv::new();
        for (name, raw) in &extracted.schemas {
            if !extracted.merge.contains(name) {
                env.insert(name.clone(), interpreter.interpret(raw));
            }
        }

        let fn_sigs: Vec<(String, Vec<(String, serde_json::Value)>, Option<serde_json::Value>)> =
            extracted
                .functions
                .iter()
                .map(|(name, sig)| {
                    (name.clone(), sig.params.clone(), sig.return_schema.clone())
                })
                .collect();
        let function_table =
            ac_typecheck_infer::FunctionTable::from_signatures(&fn_sigs, interpreter);

        let inheritable_schemas: Vec<(String, serde_json::Value)> = extracted
            .schemas
            .iter()
            .filter(|(name, _)| extracted.inheritable.contains(name))
            .cloned()
            .collect();

        let base = compilation.source.base_path();

        let mut errors = Vec::new();

        errors.extend(ac_typecheck_key_expr::check(
            &extracted.schemas,
            &env,
            &function_table,
            interpreter,
        ));

        errors.extend(ac_typecheck_key_arg::check(
            &extracted.schemas,
            interpreter,
        ));

        errors.extend(ac_typecheck_key_merge::check(
            &extracted.schemas,
            interpreter,
            &env,
        ));

        errors.extend(ac_typecheck_key_inherit::check(
            &inheritable_schemas,
            &data,
            interpreter,
        ));

        errors.extend(ac_typecheck_key_mutable::check(
            &extracted.immutable,
            &data,
        ));

        errors.extend(ac_typecheck_import::check_imports(
            &extracted.schemas,
            &extracted.merge,
            &data,
            config.resolver.as_ref(),
            interpreter,
            &env,
            base.as_deref(),
        ));

        compilation.type_errors = errors;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
