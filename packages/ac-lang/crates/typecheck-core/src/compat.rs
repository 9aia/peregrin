use crate::{Type, Variance};

/// Check if `actual` is compatible with `expected`.
///
/// Rules:
/// - `Unknown` is compatible with anything (and vice versa).
/// - `Never` is compatible with anything (it's uninhabited).
/// - `Var` is compatible with `Unknown`/`Never` (graceful degradation)
///   and with an identical `Var` (same name). Outside unification,
///   `Var` is **not** compatible with concrete types.
/// - Exact matches are compatible.
/// - `Number` accepts `Number` (no integer subtyping yet).
/// - `Array<T>` is compatible if item types are compatible.
/// - `Object` is compatible if every expected property exists with a
///   compatible type. Extra properties in `actual` are allowed.
/// - `Function` is compatible if param types are contravariantly compatible
///   and return types are covariantly compatible.
pub fn is_compatible(actual: &Type, expected: &Type) -> bool {
    match (actual, expected) {
        (_, Type::Unknown) | (Type::Unknown, _) => true,
        (_, Type::Never) | (Type::Never, _) => true,

        (Type::Var(a), Type::Var(e)) => a == e,

        (Type::Trait(a), Type::Trait(e)) => a == e,

        (Type::App(a_ctor, a_args), Type::App(e_ctor, e_args)) => {
            is_compatible(a_ctor, e_ctor)
                && a_args.len() == e_args.len()
                && a_args.iter().zip(e_args).all(|(a, e)| is_compatible(a, e))
        }

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
            // Function params are contravariant: actual params must accept
            // everything expected params accept.
            a_fn.params
                .iter()
                .zip(&e_fn.params)
                .all(|((_, a_ty), (_, e_ty))| is_compatible(e_ty, a_ty))
                // Return types are covariant: actual return must be compatible
                // with expected return.
                && is_compatible(&a_fn.return_type, &e_fn.return_type)
        }

        _ => false,
    }
}

/// Check if `actual` is compatible with `expected` under a variance annotation.
///
/// - [`Variance::Covariant`]: `is_compatible(actual, expected)` — actual
///   can be a subtype of expected.
/// - [`Variance::Contravariant`]: `is_compatible(expected, actual)` — flip
///   the direction (used for function parameter positions).
/// - [`Variance::Invariant`]: `actual == expected` (exact match required).
/// - [`Variance::Bivariant`]: always compatible.
///
/// `Unknown` and `Never` are always compatible regardless of variance.
pub fn variance_compatible(actual: &Type, expected: &Type, variance: Variance) -> bool {
    if matches!(actual, Type::Unknown) || matches!(expected, Type::Unknown)
        || matches!(actual, Type::Never) || matches!(expected, Type::Never)
    {
        return true;
    }
    match variance {
        Variance::Covariant => is_compatible(actual, expected),
        Variance::Contravariant => is_compatible(expected, actual),
        Variance::Invariant => actual == expected,
        Variance::Bivariant => true,
    }
}
