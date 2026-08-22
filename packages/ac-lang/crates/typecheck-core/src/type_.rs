use std::collections::BTreeMap;

/// A static type in the AC type system.
///
/// Types are derived from schema fragments via a [`SchemaInterpreter`].
/// The typechecker compares inferred types against declared types to
/// detect mismatches before runtime evaluation.
///
/// `Unknown` is compatible with everything — it represents types that
/// cannot be statically determined. `Never` represents impossible code
/// paths (e.g., a function that always errors).
///
/// `Var` represents a type variable used in generic function signatures.
/// Outside of unification (during generic call instantiation), `Var` is
/// not compatible with concrete types. See [`crate::unify`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    String,
    Number,
    Boolean,
    Null,
    Array(Box<Type>),
    Object(BTreeMap<String, Type>),
    Function(FunctionType),
    /// A type variable (e.g. `T`, `U`) used in generic function signatures.
    Var(String),
    Unknown,
    Never,
}

/// A function type — type parameters, parameter names/types, and return type.
///
/// `type_params` is empty for monomorphic functions. When non-empty, the
/// function is generic and the typechecker unifies argument types against
/// parameter types to instantiate the return type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionType {
    /// Type parameter names (e.g. `["T", "U"]`). Empty for monomorphic functions.
    pub type_params: Vec<String>,
    /// `(param_name, param_type)` pairs, in declaration order.
    pub params: Vec<(String, Type)>,
    /// The return type.
    pub return_type: Box<Type>,
}

impl Type {
    /// Human-readable name for error messages.
    pub fn name(&self) -> String {
        match self {
            Type::String => "string".to_string(),
            Type::Number => "number".to_string(),
            Type::Boolean => "boolean".to_string(),
            Type::Null => "null".to_string(),
            Type::Array(item) => format!("array<{}>", item.name()),
            Type::Object(props) => {
                let inner = props
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", v.name()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("object{{{inner}}}")
            }
            Type::Function(f) => {
                let params = f
                    .params
                    .iter()
                    .map(|(n, t)| format!("{n}: {}", t.name()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("fn({params}) -> {}", f.return_type.name())
            }
            Type::Var(name) => name.clone(),
            Type::Unknown => "unknown".to_string(),
            Type::Never => "never".to_string(),
        }
    }

    /// Returns `true` if this type is `Unknown`.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Type::Unknown)
    }
}
