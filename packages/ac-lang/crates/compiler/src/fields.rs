use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::Path;

use ac_dialect::Dialect;
use ac_dialect::Schema;
use ac_dialect_canonical::AC_DIALECT;
use ac_fields;
use ac_frontend::frontend_for;
use ac_host::DefaultHost;
use ac_runtime::NonFunctionValue;
use ac_typecheck_core::{SchemaInterpreter, TypeEnv};
use ac_typecheck_infer::{FunctionTable, HostFunctionSigs};
use ac_typecheck_schema_json::JsonSchemaInterpreter;

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

    let extracted = ac_fields::extract_fields(value, &AC_DIALECT)?;

    if extracted.schemas.is_empty() {
        return Err("no `$:` section found in input".to_string());
    }

    let field_schemas = extracted.schemas;
    let inheritable = extracted.inheritable;
    let immutable = extracted.immutable;
    let computed = extracted.computed;
    let merge = extracted.merge;
    let data = extracted.data;

    let mut namespaces = ac_fields::Namespaces::new();
    namespaces.insert(
        ac_fields::DEFAULT_NAMESPACE.to_string(),
        ac_fields::build_doc_namespace(&data),
    );
    namespaces.insert("env".to_string(), ac_fields::build_env_namespace());

    let host = DefaultHost::default();
    let file_resolver = ac_module_resolver_file::FileResolver::with_cache(
        ac_module_resolver_memory::MemoryModuleCache::new(),
    );

    // Build typechecker context for generic type arg resolution.
    // This mirrors what TypecheckPass does: interpret schemas into TypeEnv
    // and build a FunctionTable from user-declared + host function sigs.
    let interpreter = JsonSchemaInterpreter;
    let mut type_env = TypeEnv::new();
    for (name, raw) in &field_schemas {
        if !merge.contains(name) {
            type_env.insert(name.clone(), interpreter.interpret(raw));
        }
    }

    let user_fn_sigs: Vec<ac_typecheck_infer::RawFnSig> = field_schemas
        .iter()
        .filter_map(|(name, raw)| {
            let type_name = raw.get("type").and_then(|v| v.as_str())?;
            if type_name != "function" {
                return None;
            }
            let type_params = raw
                .get("typeParams")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| {
                            if let Some(s) = v.as_str() {
                                return Some(ac_typecheck_core::TypeParam::new(s));
                            }
                            let obj = v.as_object()?;
                            let name = obj.get("name")?.as_str()?.to_string();
                            Some(ac_typecheck_core::TypeParam::new(name))
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let params = raw
                .get("params")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|p| {
                            let pname = p.get("name")?.as_str()?.to_string();
                            let pschema = p.get("schema").cloned().unwrap_or(serde_json::Value::Null);
                            Some((pname, pschema))
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let return_schema = raw.get("return").cloned();
            Some((name.clone(), type_params, params, return_schema))
        })
        .collect();

    let host_sigs = HostFunctionSigs::default_host();
    let mut all_sigs = user_fn_sigs;
    all_sigs.extend(host_sigs.0.iter().cloned());
    let function_table = FunctionTable::from_signatures(&all_sigs, &interpreter);

    let resolver = VmResolver {
        host,
        namespaces: namespaces.clone(),
        depth: Cell::new(0),
        call_chain: RefCell::new(Vec::new()),
        base_path: base_path.map(|p| p.to_path_buf()),
        import_cache: RefCell::new(HashMap::new()),
        resolver: Some(&file_resolver),
        type_env: Some(type_env),
        function_table: Some(function_table),
    };

    let interpolated = ac_fields::interpolate_scoped(
        data.inner(),
        &namespaces,
        &inheritable,
        &resolver,
        &AC_DIALECT,
    );
    let mut data = NonFunctionValue::new(interpolated);

    let mut errors = Vec::new();

    let compute_errors = ac_fields::compute_fields(
        data.inner_mut(),
        &computed,
        &mut namespaces,
        &resolver,
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
        .collect::<Result<Vec<_>, _>>()?;

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
