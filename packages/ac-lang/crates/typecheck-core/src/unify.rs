use std::collections::BTreeMap;

use crate::Type;

/// A substitution map: type variable name → concrete type.
pub type Substitution = BTreeMap<String, Type>;

/// Unify `actual` against `pattern`, updating `subst` in place.
///
/// Returns `true` if unification succeeds. On failure, `subst` may contain
/// partial bindings from before the failure — callers should discard it.
///
/// ## Rules
///
/// - `Unknown` / `Never` in either position: compatible with anything.
/// - `Var(name)` in **pattern**: bind `name → actual` (or check consistency
///   if already bound). This is the primary direction — the pattern is the
///   generic signature, `actual` is the concrete argument type.
/// - `Var(name)` in **actual** but not pattern: only compatible if pattern
///   is `Unknown`/`Never` (handled by the catch-all). A concrete pattern
///   cannot unify with a variable in the actual position.
/// - Identical `Var` (same name) in both: compatible.
/// - Concrete types: structural recursion into `Array`, `Object`, `Function`.
///
/// ## Type parameter scoping
///
/// `type_params` is the set of variable names that are considered "in scope"
/// for this unification — only these `Var`s are eligible for binding. A
/// `Var` not in `type_params` is treated as an opaque type (compatible only
/// with itself or `Unknown`/`Never`).
pub fn unify(
    actual: &Type,
    pattern: &Type,
    subst: &mut Substitution,
    type_params: &[String],
) -> bool {
    match (actual, pattern) {
        (_, Type::Unknown) | (Type::Unknown, _) => true,
        (_, Type::Never) | (Type::Never, _) => true,

        (Type::Var(a), Type::Var(e)) => a == e,

        // Pattern is a type variable in scope → bind or check consistency.
        (actual, Type::Var(name)) if type_params.contains(name) => {
            match subst.get(name) {
                None => {
                    subst.insert(name.clone(), actual.clone());
                    true
                }
                Some(existing) => {
                    // Already bound — check consistency.
                    // If existing is Unknown, update to the more specific type.
                    if matches!(existing, Type::Unknown) {
                        subst.insert(name.clone(), actual.clone());
                        return true;
                    }
                    if matches!(actual, Type::Unknown) {
                        return true;
                    }
                    types_match(existing, actual)
                }
            }
        }

        // Pattern is a Var not in type_params — opaque, only matches itself
        // (handled by the Var vs Var arm above) or Unknown/Never (handled above).
        (_, Type::Var(_)) => false,

        // Actual is a Var not in pattern position — can't unify with concrete.
        (Type::Var(_), _) => false,

        (Type::String, Type::String) => true,
        (Type::Number, Type::Number) => true,
        (Type::Boolean, Type::Boolean) => true,
        (Type::Null, Type::Null) => true,

        (Type::Array(a_item), Type::Array(e_item)) => {
            unify(a_item, e_item, subst, type_params)
        }

        (Type::Object(a_props), Type::Object(e_props)) => {
            e_props.iter().all(|(key, e_ty)| {
                a_props
                    .get(key)
                    .map(|a_ty| unify(a_ty, e_ty, subst, type_params))
                    .unwrap_or(false)
            })
        }

        (Type::Function(a_fn), Type::Function(e_fn)) => {
            if a_fn.params.len() != e_fn.params.len() {
                return false;
            }
            // Unify param types (contravariant position — but we check
            // structural match for simplicity, same as is_compatible).
            a_fn
                .params
                .iter()
                .zip(&e_fn.params)
                .all(|((_, a_ty), (_, e_ty))| unify(a_ty, e_ty, subst, type_params))
                && unify(&a_fn.return_type, &e_fn.return_type, subst, type_params)
        }

        _ => false,
    }
}

/// Check if two types are structurally identical (no unification, no Unknown
/// wildcard). Used for consistency checks on already-bound variables.
fn types_match(a: &Type, b: &Type) -> bool {
    match (a, b) {
        (Type::String, Type::String) => true,
        (Type::Number, Type::Number) => true,
        (Type::Boolean, Type::Boolean) => true,
        (Type::Null, Type::Null) => true,
        (Type::Var(a_name), Type::Var(b_name)) => a_name == b_name,
        (Type::Array(a_item), Type::Array(b_item)) => types_match(a_item, b_item),
        (Type::Object(a_props), Type::Object(b_props)) => {
            a_props.len() == b_props.len()
                && a_props.iter().all(|(k, v)| {
                    b_props.get(k).map(|bv| types_match(v, bv)).unwrap_or(false)
                })
        }
        (Type::Function(a_fn), Type::Function(b_fn)) => {
            a_fn.params.len() == b_fn.params.len()
                && a_fn
                    .params
                    .iter()
                    .zip(&b_fn.params)
                    .all(|((_, at), (_, bt))| types_match(at, bt))
                && types_match(&a_fn.return_type, &b_fn.return_type)
        }
        _ => false,
    }
}

/// Apply a substitution to a type, replacing all bound `Var`s with their
/// concrete types. Unbound `Var`s are left as-is.
pub fn substitute(ty: &Type, subst: &Substitution) -> Type {
    match ty {
        Type::Var(name) => subst.get(name).cloned().unwrap_or_else(|| ty.clone()),

        Type::Array(item) => Type::Array(Box::new(substitute(item, subst))),

        Type::Object(props) => {
            Type::Object(props.iter().map(|(k, v)| (k.clone(), substitute(v, subst))).collect())
        }

        Type::Function(f) => Type::Function(crate::FunctionType {
            type_params: f.type_params.clone(),
            params: f
                .params
                .iter()
                .map(|(n, t)| (n.clone(), substitute(t, subst)))
                .collect(),
            return_type: Box::new(substitute(&f.return_type, subst)),
        }),

        _ => ty.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FunctionType;

    #[test]
    fn bind_var_to_concrete() {
        let mut subst = Substitution::new();
        assert!(unify(
            &Type::String,
            &Type::Var("T".to_string()),
            &mut subst,
            &["T".to_string()],
        ));
        assert_eq!(subst.get("T"), Some(&Type::String));
    }

    #[test]
    fn consistent_rebinding_succeeds() {
        let mut subst = Substitution::new();
        let tp = ["T".to_string()];
        assert!(unify(&Type::String, &Type::Var("T".to_string()), &mut subst, &tp));
        assert!(unify(&Type::String, &Type::Var("T".to_string()), &mut subst, &tp));
        assert_eq!(subst.get("T"), Some(&Type::String));
    }

    #[test]
    fn inconsistent_rebinding_fails() {
        let mut subst = Substitution::new();
        let tp = ["T".to_string()];
        assert!(unify(&Type::String, &Type::Var("T".to_string()), &mut subst, &tp));
        assert!(!unify(&Type::Number, &Type::Var("T".to_string()), &mut subst, &tp));
    }

    #[test]
    fn unknown_actual_succeeds_without_binding() {
        let mut subst = Substitution::new();
        // Unknown in actual position short-circuits via the catch-all —
        // unification succeeds but no binding is created.
        assert!(unify(
            &Type::Unknown,
            &Type::Var("T".to_string()),
            &mut subst,
            &["T".to_string()],
        ));
        assert!(subst.is_empty());
    }

    #[test]
    fn unknown_existing_allows_more_specific() {
        let mut subst = Substitution::new();
        let tp = ["T".to_string()];
        assert!(unify(&Type::Unknown, &Type::Var("T".to_string()), &mut subst, &tp));
        assert!(unify(&Type::String, &Type::Var("T".to_string()), &mut subst, &tp));
        assert_eq!(subst.get("T"), Some(&Type::String));
    }

    #[test]
    fn nested_array_unify() {
        let mut subst = Substitution::new();
        assert!(unify(
            &Type::Array(Box::new(Type::String)),
            &Type::Array(Box::new(Type::Var("T".to_string()))),
            &mut subst,
            &["T".to_string()],
        ));
        assert_eq!(subst.get("T"), Some(&Type::String));
    }

    #[test]
    fn function_unify_binds_vars() {
        let mut subst = Substitution::new();
        let tp = ["T".to_string()];

        let actual = Type::Function(FunctionType {
            type_params: vec![],
            params: vec![("x".to_string(), Type::String)],
            return_type: Box::new(Type::String),
        });

        let pattern = Type::Function(FunctionType {
            type_params: vec![],
            params: vec![("x".to_string(), Type::Var("T".to_string()))],
            return_type: Box::new(Type::Var("T".to_string())),
        });

        assert!(unify(&actual, &pattern, &mut subst, &tp));
        assert_eq!(subst.get("T"), Some(&Type::String));
    }

    #[test]
    fn var_not_in_type_params_does_not_bind() {
        let mut subst = Substitution::new();
        // T is not in type_params, so Var("T") in pattern is opaque
        assert!(!unify(
            &Type::String,
            &Type::Var("T".to_string()),
            &mut subst,
            &[],
        ));
        assert!(subst.is_empty());
    }

    #[test]
    fn same_var_name_compatible() {
        let mut subst = Substitution::new();
        assert!(unify(
            &Type::Var("T".to_string()),
            &Type::Var("T".to_string()),
            &mut subst,
            &["T".to_string()],
        ));
        // No binding created — they're the same variable
        assert!(subst.is_empty());
    }

    #[test]
    fn substitute_replaces_bound_vars() {
        let mut subst = Substitution::new();
        subst.insert("T".to_string(), Type::String);
        subst.insert("U".to_string(), Type::Number);

        let ty = Type::Array(Box::new(Type::Var("T".to_string())));
        assert_eq!(substitute(&ty, &subst), Type::Array(Box::new(Type::String)));

        let ty = Type::Function(FunctionType {
            type_params: vec![],
            params: vec![("x".to_string(), Type::Var("T".to_string()))],
            return_type: Box::new(Type::Var("U".to_string())),
        });
        let result = substitute(&ty, &subst);
        if let Type::Function(f) = result {
            assert_eq!(f.params[0].1, Type::String);
            assert_eq!(*f.return_type, Type::Number);
        } else {
            panic!("expected function type");
        }
    }

    #[test]
    fn substitute_leaves_unbound_vars() {
        let subst = Substitution::new();
        let ty = Type::Var("T".to_string());
        assert_eq!(substitute(&ty, &subst), Type::Var("T".to_string()));
    }

    #[test]
    fn two_type_params_independent() {
        let mut subst = Substitution::new();
        let tp = ["T".to_string(), "U".to_string()];

        // First arg binds T=String
        assert!(unify(
            &Type::String,
            &Type::Var("T".to_string()),
            &mut subst,
            &tp,
        ));
        // Second arg binds U=Number
        assert!(unify(
            &Type::Number,
            &Type::Var("U".to_string()),
            &mut subst,
            &tp,
        ));

        assert_eq!(subst.get("T"), Some(&Type::String));
        assert_eq!(subst.get("U"), Some(&Type::Number));
    }

    #[test]
    fn map_signature_unify() {
        let mut subst = Substitution::new();
        let tp = ["T".to_string(), "U".to_string()];

        // map<T, U>(Array<T>, fn(T) -> U) -> Array<U>
        // Call: map(string_array, str_to_num)
        // Arg 1: Array<String> unify Array<T>  →  T = String
        assert!(unify(
            &Type::Array(Box::new(Type::String)),
            &Type::Array(Box::new(Type::Var("T".to_string()))),
            &mut subst,
            &tp,
        ));
        // Arg 2: fn(String) -> Number unify fn(T) -> U  →  T = String (consistent), U = Number
        let actual_fn = Type::Function(FunctionType {
            type_params: vec![],
            params: vec![("x".to_string(), Type::String)],
            return_type: Box::new(Type::Number),
        });
        let pattern_fn = Type::Function(FunctionType {
            type_params: vec![],
            params: vec![("x".to_string(), Type::Var("T".to_string()))],
            return_type: Box::new(Type::Var("U".to_string())),
        });
        assert!(unify(&actual_fn, &pattern_fn, &mut subst, &tp));

        assert_eq!(subst.get("T"), Some(&Type::String));
        assert_eq!(subst.get("U"), Some(&Type::Number));

        // Return type: Array<U> → Array<Number>
        let return_ty = Type::Array(Box::new(Type::Var("U".to_string())));
        assert_eq!(substitute(&return_ty, &subst), Type::Array(Box::new(Type::Number)));
    }
}
