use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use ac_dialect::{Dialect, Schema};
use ac_dialect_canonical::AC_DIALECT;
use ac_fields;
use ac_runtime_example_1::DefaultHost;
use ac_runtime_values::NonFunctionValue;

use crate::{
    Config, DriverError, DriverPipeline, ExtractedDoc, Session, Source, Stage, default_cli_config,
    ExtractStage, ParseStage,
};
use crate::resolver::{CompilerHost, Import};

/// Report from a fields validation run.
pub struct FieldsReport {
    /// `(field_name, type_name)` pairs for declared fields (excluding merge fields).
    pub field_types: Vec<(String, String)>,
    /// Field names that validated successfully.
    pub ok: Vec<String>,
    /// Validation error messages.
    pub errors: Vec<String>,
    /// Undeclared field names found in the data.
    pub undeclared: Vec<String>,
}

impl FieldsReport {
    pub fn error_count(&self) -> usize {
        self.errors.len() + self.undeclared.len()
    }

    pub fn is_valid(&self) -> bool {
        self.error_count() == 0
    }
}

/// Fields phase: `ExtractedDoc → FieldsReport`.
///
/// Interpolates, computes, merges, and validates the `$:` field
/// declarations against the document data.
pub struct FieldsStage {
    pub base_path: Option<PathBuf>,
}

impl Stage<Config> for FieldsStage {
    type Input = ExtractedDoc;
    type Output = FieldsReport;

    fn name(&self) -> &'static str {
        "fields"
    }

    fn run(&mut self, doc: ExtractedDoc, _session: &mut Session<Config>) -> Result<FieldsReport, DriverError> {
        if doc.extracted.schemas.is_empty() {
            return Err(DriverError::new("no `$:` section found in input"));
        }

        let field_schemas = doc.extracted.schemas;
        let inheritable = doc.extracted.inheritable;
        let immutable = doc.extracted.immutable;
        let computed = doc.extracted.computed;
        let merge = doc.extracted.merge;
        let base_data = doc.extracted.data;

        let mut namespaces = ac_fields::Namespaces::new();
        namespaces.insert(
            ac_fields::DEFAULT_NAMESPACE.to_string(),
            ac_fields::build_doc_namespace(&base_data),
        );
        namespaces.insert("env".to_string(), ac_fields::build_env_namespace());

        let host = DefaultHost::default();
        let file_resolver = ac_module_resolver_file::FileResolver::with_cache(
            ac_module_resolver_memory::MemoryModuleCache::new(),
        );

        let import = Import::new(
            self.base_path.clone(),
            Rc::new(Box::new(file_resolver) as Box<dyn ac_module_resolver_core::ModuleResolver>),
            Rc::new(RefCell::new(HashMap::new())),
        );
        let host = CompilerHost::new(host, import, &namespaces);

        let interpolated =
            ac_fields::interpolate_scoped(base_data.inner(), &namespaces, &inheritable, &host, &AC_DIALECT);
        let mut data = NonFunctionValue::new(interpolated);

        let mut errors = Vec::new();

        let compute_errors = ac_fields::compute_fields(
            data.inner_mut(),
            &computed,
            &mut namespaces,
            &host,
            &AC_DIALECT,
        );
        errors.extend(compute_errors);

        let merge_errors = ac_fields::merge_fields(data.inner_mut(), &merge);
        errors.extend(merge_errors);

        let vocab = AC_DIALECT.schema_vocabulary();
        let schemas: Vec<(String, Box<dyn Schema<NonFunctionValue>>)> = field_schemas
            .iter()
            .filter(|(name, _)| !merge.iter().any(|m| m == name))
            .map(|(name, raw)| {
                let compiled = vocab
                    .compile(raw)
                    .map_err(|e| format!("{name}: invalid schema — {e}"))?;
                Ok::<_, String>((name.clone(), compiled))
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(DriverError::new)?;

        let mut field_types = Vec::new();
        let mut ok = Vec::new();

        for (name, schema) in &schemas {
            let type_name = ac_dialect_canonical::JsonSchema::new(
                field_schemas
                    .iter()
                    .find(|(n, _)| n == name)
                    .unwrap()
                    .1
                    .clone(),
            )
            .type_name()
            .unwrap_or("any")
            .to_string();
            field_types.push((name.clone(), type_name));

            let field_value = data.get(name).unwrap_or(&serde_json::Value::Null);
            let field_rv = NonFunctionValue::new(field_value.clone());
            match schema.validate(&field_rv) {
                Ok(()) => ok.push(name.clone()),
                Err(e) => errors.push(format!("{name}: INVALID — {e}")),
            }
        }

        let declared: Vec<&str> = schemas.iter().map(|(n, _)| n.as_str()).collect();
        let undeclared: Vec<String> = ac_fields::undeclared_fields(&data, &declared)
            .into_iter()
            .map(|name| format!("{name}: UNDECLARED — not in $:"))
            .collect();

        for msg in ac_fields::validate_mutable(data.inner(), &immutable) {
            errors.push(msg);
        }

        Ok(FieldsReport {
            field_types,
            ok,
            errors,
            undeclared,
        })
    }
}

/// Validate `$:` fields through the parse → extract → fields pipeline.
pub fn run(input: &str, format: &str, base_path: Option<&Path>) -> Result<FieldsReport, String> {
    let source = Source::new(input, format);
    let fields = FieldsStage {
        base_path: base_path.map(|p| p.to_path_buf()),
    };
    let mut pipeline = DriverPipeline::new(ParseStage)
        .then(ExtractStage)
        .then(fields);
    let mut session = Session::new(default_cli_config());
    pipeline
        .run(source, &mut session)
        .map_err(|e| e.to_string())
}
