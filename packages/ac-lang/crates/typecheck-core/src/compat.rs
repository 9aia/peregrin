use crate::Type;

/// Check if `actual` is compatible with `expected`.
///
/// Rules:
/// - `Unknown` is compatible with anything (and vice versa).
/// - `Never` is compatible with anything (it's uninhabited).
/// - Exact matches are compatible.
/// - `Number` accepts `Number` (no integer subtyping yet).
/// - `Array<T>` is compatible if item types are compatible.
/// - `Object` is compatible if every expected property exists with a
///   compatible type. Extra properties in `actual` are allowed.
/// - `Function` is compatible if param and return types are compatible.
pub fn is_compatible(actual: &Type, expected: &Type) -> bool {
    match (actual, expected) {
        (_, Type::Unknown) | (Type::Unknown, _) => true,
        (_, Type::Never) | (Type::Never, _) => true,

        (Type::String, Type::String) => true,
        (Type::Number, Type::Number) => true,
        (Type::Boolean, Type::Boolean) => true,
        (Type::Null, Type::Null) => true,

        (Type::Array(a_item), Type::Array(e_item)) => is_compatible(a_item, e_item),

        (Type::Object(a_props), Type::Object(e_props)) => e_props.iter().all(|(key, e_ty)| {
            a_props
                .get(key)
                .map(|a_ty| is_compatible(a_ty, e_ty))
                .unwrap_or(false)
        }),

        (Type::Function(a_fn), Type::Function(e_fn)) => {
            if a_fn.params.len() != e_fn.params.len() {
                return false;
            }
            a_fn.params
                .iter()
                .zip(&e_fn.params)
                .all(|((_, a_ty), (_, e_ty))| is_compatible(a_ty, e_ty))
                && is_compatible(&a_fn.return_type, &e_fn.return_type)
        }

        _ => false,
    }
}
