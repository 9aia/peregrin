//! # ac-typecheck-import
//!
//! Cross-module type checking for the AC typechecker.
//!
//! This crate follows `import()` expressions in merge fields, resolves
//! them via a [`ModuleResolver`], builds [`TypeEnv`]s from imported
//! schemas, and checks type compatibility across module boundaries.
//!
//! It does **not** do any I/O directly — that's delegated to the
//! resolver implementation. This makes import type checking testable
//! without touching the filesystem.
//!
//! ## Architectural rule
//!
//! > **Import type checking uses a resolver; the resolver does the I/O.**
//!
//! The orchestrator (CLI, build tool) calls them in order:
//!
//! ```ignore
//! // 1. Build TypeEnv from local schemas
//! let env = build_env(&schemas, &interpreter);
//!
//! // 2. Check local keys (expr, arg, merge, inherit, mutable)
//! // ... (via key crates)
//!
//! // 3. Check imports (via resolver)
//! ac_typecheck_import::check_imports(
//!     &schemas, &merge_fields, &data,
//!     &resolver, &interpreter, &env, None,
//! );
//! ```

mod detect;

use std::path::Path;

use serde_json::Value as JsonValue;

use ac_resolver_core::ModuleResolver;
use ac_typecheck_core::{is_compatible, SchemaInterpreter, TypeEnv, TypeError};

/// Check type compatibility across imported modules.
///
/// For each merge field with an `import()` expression:
/// 1. Resolves the module via `resolver`.
/// 2. Builds a [`TypeEnv`] from the imported module's schemas.
/// 3. Checks that imported field types are compatible with the parent
///    scope's declared types for the same field names.
///
/// Recursively checks nested imports from imported modules.
///
/// Returns a list of [`TypeError`]s for any incompatibilities.
pub fn check_imports(
    schemas: &[(String, JsonValue)],
    merge_fields: &[String],
    data: &JsonValue,
    resolver: &dyn ModuleResolver,
    interpreter: &dyn SchemaInterpreter,
    parent_env: &TypeEnv,
    base: Option<&Path>,
) -> Vec<TypeError> {
    let mut visited = std::collections::HashSet::new();
    check_imports_inner(
        schemas,
        merge_fields,
        data,
        resolver,
        interpreter,
        parent_env,
        base,
        &mut visited,
    )
}

fn check_imports_inner(
    schemas: &[(String, JsonValue)],
    merge_fields: &[String],
    data: &JsonValue,
    resolver: &dyn ModuleResolver,
    interpreter: &dyn SchemaInterpreter,
    parent_env: &TypeEnv,
    base: Option<&Path>,
    visited: &mut std::collections::HashSet<String>,
) -> Vec<TypeError> {
    let mut errors = Vec::new();

    for field_name in merge_fields {
        let Some((_, raw)) = schemas.iter().find(|(n, _)| n == field_name) else {
            continue;
        };

        let import_path = detect_import(raw, data, field_name);
        let Some(path) = import_path else { continue };

        if visited.contains(&path) {
            continue;
        }
        visited.insert(path.clone());

        let Some(module) = resolver.resolve(&path, base) else {
            errors.push(TypeError::custom(
                field_name,
                format!("cannot resolve import: {path}"),
                "",
            ));
            continue;
        };

        let imported_schemas: Vec<(String, JsonValue)> = module
            .schemas
            .iter()
            .filter(|(name, _)| !module.merge.contains(name))
            .cloned()
            .collect();

        for (name, schema) in &imported_schemas {
            let imported_type = interpreter.interpret(schema);
            if let Some(parent_type) = parent_env.lookup(name) {
                if !imported_type.is_unknown() && !is_compatible(&imported_type, parent_type) {
                    errors.push(TypeError::mismatch(
                        name,
                        parent_type.clone(),
                        imported_type,
                        &format!("import:{path}"),
                    ));
                }
            }
        }

        if !module.merge.is_empty() {
            let nested_errors = check_imports_inner(
                &module.schemas,
                &module.merge,
                &module.data,
                resolver,
                interpreter,
                parent_env,
                module.base_path.as_deref(),
                visited,
            );
            errors.extend(nested_errors);
        }
    }

    errors
}

use detect::detect_import;

#[cfg(test)]
mod lib_tests;
