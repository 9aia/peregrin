use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::Path;

use ac_fields;
use ac_frontend::frontend_for;
use ac_host::DefaultHost;
use ac_runtime::RuntimeValue;
use ac_schema::Schema;
use ac_schema_json::JsonSchema;

use crate::resolver::VmResolver;

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

pub fn run(input: &str, format: &str, base_path: Option<&Path>) -> Result<FieldsReport, String> {
    let frontend = frontend_for(format)?;
    let value = frontend.parse(input)?;

    let extracted = ac_fields::extract_fields(value)?;

    if extracted.schemas.is_empty() {
        return Err("no `fields:` section found in input".to_string());
    }

    let field_schemas = extracted.schemas;
    let function_sigs = extracted.functions;
    let inheritable = extracted.inheritable;
    let immutable = extracted.immutable;
    let computed = extracted.computed;
    let merge = extracted.merge;
    let data = extracted.data;

    let mut namespaces = ac_fields::Namespaces::new();
    namespaces.insert(ac_fields::DEFAULT_NAMESPACE.to_string(), ac_fields::build_doc_namespace(&data));
    namespaces.insert("env".to_string(), ac_fields::build_env_namespace());

    let host = DefaultHost::default();
    let file_resolver = ac_resolver_file::FileResolver::with_cache(
        ac_resolver_core::MemoryModuleCache::new(),
    );
    let resolver = VmResolver {
        host,
        function_sigs: &function_sigs,
        namespaces: namespaces.clone(),
        depth: Cell::new(0),
        base_path: base_path.map(|p| p.to_path_buf()),
        import_cache: RefCell::new(HashMap::new()),
        imported_functions: RefCell::new(Vec::new()),
        resolver: Some(&file_resolver),
    };

    let interpolated = ac_fields::interpolate_scoped(
        data.inner(),
        &namespaces,
        &inheritable,
        &resolver,
    );
    let mut data = RuntimeValue::new(interpolated);

    let mut errors = Vec::new();

    let compute_errors =
        ac_fields::compute_fields(data.inner_mut(), &computed, &mut namespaces, &resolver);
    errors.extend(compute_errors);

    let merge_errors = ac_fields::merge_fields(data.inner_mut(), &merge);
    errors.extend(merge_errors);

    let schemas: Vec<(String, JsonSchema)> = field_schemas
        .iter()
        .filter(|(name, _)| !merge.iter().any(|m| m == name))
        .map(|(name, raw)| (name.clone(), JsonSchema::new(raw.clone())))
        .collect();

    let mut field_types = Vec::new();
    let mut ok = Vec::new();

    for (name, schema) in &schemas {
        let type_name = schema.type_name().unwrap_or("any").to_string();
        field_types.push((name.clone(), type_name));

        let field_value = data.get(name).unwrap_or(&serde_json::Value::Null);
        let field_rv = RuntimeValue::new(field_value.clone());
        match schema.validate(&field_rv) {
            Ok(()) => ok.push(name.clone()),
            Err(e) => errors.push(format!("{name}: INVALID — {e}")),
        }
    }

    let declared: Vec<&str> = schemas.iter().map(|(n, _)| n.as_str()).collect();
    let undeclared: Vec<String> = ac_fields::undeclared_fields(&data, &declared)
        .into_iter()
        .map(|name| format!("{name}: UNDECLARED — not in fields:"))
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
