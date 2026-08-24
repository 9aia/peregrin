use std::collections::BTreeMap;

use crate::{FunctionType, Type, TypeParam, Variance, is_compatible};

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
/// `type_params` is the set of type parameter declarations that are
/// considered "in scope" for this unification — only `Var`s whose name
/// matches a `TypeParam.name` are eligible for binding. A `Var` not in
/// `type_params` is treated as an opaque type (compatible only with
/// itself or `Unknown`/`Never`).
///
/// ## Trait bounds
///
/// This function does **not** check trait bounds — it only handles binding.
/// Use [`check_bounds`] after unification to verify that bound types
/// satisfy their declared bounds.
pub fn unify(
    actual: &Type,
    pattern: &Type,
    subst: &mut Substitution,
    type_params: &[TypeParam],
) -> bool {
    match (actual, pattern) {
        (_, Type::Unknown) | (Type::Unknown, _) => true,
        (_, Type::Never) | (Type::Never, _) => true,

        (Type::Var(a), Type::Var(e)) => a == e,

        // Pattern is a type variable in scope → bind or check consistency.
        (actual, Type::Var(name)) => {
            let Some(tp) = type_params.iter().find(|tp| &tp.name == name) else {
                return false;
            };
            let variance = tp.variance;
            match subst.get(name) {
                None => {
                    subst.insert(name.clone(), actual.clone());
                    true
                }
                Some(existing) => {
                    // Already bound — check consistency using variance.
                    // If existing is Unknown, update to the more specific type.
                    if matches!(existing, Type::Unknown) {
                        subst.insert(name.clone(), actual.clone());
                        return true;
                    }
                    if matches!(actual, Type::Unknown) {
                        return true;
                    }
                    // Variance-aware consistency check:
                    // - Invariant: exact match required
                    // - Covariant: actual must be compatible with existing
                    // - Contravariant: existing must be compatible with actual
                    // - Bivariant: always consistent
                    match variance {
                        Variance::Invariant => types_match(existing, actual),
                        Variance::Covariant => is_compatible(actual, existing),
                        Variance::Contravariant => is_compatible(existing, actual),
                        Variance::Bivariant => true,
                    }
                }
            }
        }

        // Actual is a Var not in pattern position — can't unify with concrete.
        (Type::Var(_), _) => false,

        (Type::String, Type::String) => true,
        (Type::Number, Type::Number) => true,
        (Type::Boolean, Type::Boolean) => true,
        (Type::Null, Type::Null) => true,

        (Type::Array(a_item), Type::Array(e_item)) => unify(a_item, e_item, subst, type_params),

        (Type::Object(a_props), Type::Object(e_props)) => e_props.iter().all(|(key, e_ty)| {
            a_props
                .get(key)
                .map(|a_ty| unify(a_ty, e_ty, subst, type_params))
                .unwrap_or(false)
        }),

        (Type::Function(a_fn), Type::Function(e_fn)) => {
            if a_fn.params.len() != e_fn.params.len() {
                return false;
            }
            // Unify param types (contravariant position — but we check
            // structural match for simplicity, same as is_compatible).
            a_fn.params
                .iter()
                .zip(&e_fn.params)
                .all(|((_, a_ty), (_, e_ty))| unify(a_ty, e_ty, subst, type_params))
                && unify(&a_fn.return_type, &e_fn.return_type, subst, type_params)
        }

        // Type application: unify constructor and arguments.
        // This supports higher-kinded types — a Var in the constructor
        // position can be bound to a concrete type constructor.
        (Type::App(a_ctor, a_args), Type::App(e_ctor, e_args)) => {
            if a_args.len() != e_args.len() {
                return false;
            }
            // Unify constructors (may bind a higher-kinded type variable).
            unify(a_ctor, e_ctor, subst, type_params)
                && a_args
                    .iter()
                    .zip(e_args)
                    .all(|(a, e)| unify(a, e, subst, type_params))
        }

        // HKT: actual is a concrete type like Array<String>, pattern is App(Var("F"), [Var("T")])
        // We need to decompose the actual type to bind both F and T.
        // Array<T> is represented as App(Array, [T]) internally for HKT matching.
        (actual, Type::App(e_ctor, e_args))
            if type_params
                .iter()
                .any(|tp| matches!(&**e_ctor, Type::Var(name) if tp.name == *name)) =>
        {
            // The pattern constructor is a type variable in scope.
            // Try to decompose the actual type into a constructor + args.
            if let Some((a_ctor, a_args)) = decompose_type(actual) {
                if a_args.len() != e_args.len() {
                    return false;
                }
                // Bind the higher-kinded type variable to the constructor.
                let ctor_name = if let Type::Var(name) = &**e_ctor {
                    name.clone()
                } else {
                    return false;
                };
                match subst.get(&ctor_name) {
                    None => {
                        subst.insert(ctor_name.clone(), a_ctor);
                    }
                    Some(existing) => {
                        if !types_match(existing, &a_ctor) {
                            return false;
                        }
                    }
                }
                // Unify the type arguments.
                a_args
                    .iter()
                    .zip(e_args)
                    .all(|(a, e)| unify(a, e, subst, type_params))
            } else {
                false
            }
        }

        _ => false,
    }
}

/// Decompose a concrete type into a constructor and its type arguments.
///
/// This is used for HKT unification — e.g., `Array<String>` decomposes into
/// `(Array, [String])`, and `Object({"a": String})` decomposes into
/// `(Object, [Object({"a": String})])` (objects are treated as nullary).
///
/// Returns `None` for types that cannot be decomposed (primitives, Unknown, etc.).
fn decompose_type(ty: &Type) -> Option<(Type, Vec<Type>)> {
    match ty {
        Type::Array(item) => Some((Type::Array(Box::new(Type::Unknown)), vec![(**item).clone()])),
        Type::App(ctor, args) => Some(((**ctor).clone(), args.clone())),
        Type::Function(f) => {
            // Functions decompose into (Function, [param_types..., return_type])
            let mut args = f.params.iter().map(|(_, t)| t.clone()).collect::<Vec<_>>();
            args.push((*f.return_type).clone());
            Some((
                Type::Function(FunctionType {
                    type_params: f.type_params.clone(),
                    params: f
                        .params
                        .iter()
                        .map(|(n, _)| (n.clone(), Type::Unknown))
                        .collect(),
                    return_type: Box::new(Type::Unknown),
                }),
                args,
            ))
        }
        _ => None,
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
        (Type::Trait(a_name), Type::Trait(b_name)) => a_name == b_name,
        (Type::App(a_ctor, a_args), Type::App(b_ctor, b_args)) => {
            types_match(a_ctor, b_ctor)
                && a_args.len() == b_args.len()
                && a_args.iter().zip(b_args).all(|(a, b)| types_match(a, b))
        }
        (Type::Array(a_item), Type::Array(b_item)) => types_match(a_item, b_item),
        (Type::Object(a_props), Type::Object(b_props)) => {
            a_props.len() == b_props.len()
                && a_props
                    .iter()
                    .all(|(k, v)| b_props.get(k).map(|bv| types_match(v, bv)).unwrap_or(false))
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

        Type::Object(props) => Type::Object(
            props
                .iter()
                .map(|(k, v)| (k.clone(), substitute(v, subst)))
                .collect(),
        ),

        Type::Function(f) => Type::Function(crate::FunctionType {
            type_params: f.type_params.clone(),
            params: f
                .params
                .iter()
                .map(|(n, t)| (n.clone(), substitute(t, subst)))
                .collect(),
            return_type: Box::new(substitute(&f.return_type, subst)),
        }),

        Type::App(ctor, args) => Type::App(
            Box::new(substitute(ctor, subst)),
            args.iter().map(|a| substitute(a, subst)).collect(),
        ),

        _ => ty.clone(),
    }
}

/// Check that all type parameter bounds are satisfied by the substitution.
///
/// After unification, each type parameter's bound type (from `TypeParam::bounds`)
/// is resolved against the substitution and checked against the concrete type
/// using the provided [`crate::TraitRegistry`].
///
/// Returns `true` if all bounds are satisfied (or if bounds are empty).
pub fn check_bounds(
    subst: &Substitution,
    type_params: &[TypeParam],
    registry: &crate::TraitRegistry,
) -> bool {
    type_params.iter().all(|tp| {
        if tp.bounds.is_empty() {
            return true;
        }
        match subst.get(&tp.name) {
            None => true,
            Some(concrete) => registry.check_bounds(concrete, &tp.bounds),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FunctionType, TraitRegistry, Variance};

    #[test]
    fn hkt_bind_constructor_var() {
        let mut subst = Substitution::new();
        let tp = [TypeParam::new("F"), TypeParam::new("T")];

        // Actual: Array<String>, Pattern: App(F, [T])
        // Should bind F = Array<Unknown> (decomposed constructor), T = String
        let actual = Type::Array(Box::new(Type::String));
        let pattern = Type::App(
            Box::new(Type::Var("F".to_string())),
            vec![Type::Var("T".to_string())],
        );

        assert!(unify(&actual, &pattern, &mut subst, &tp));
        // F should be bound to the Array constructor
        assert!(subst.contains_key("F"));
        assert!(subst.contains_key("T"));
        assert_eq!(subst.get("T"), Some(&Type::String));
    }

    #[test]
    fn hkt_app_unify_both_concrete() {
        let mut subst = Substitution::new();
        let tp = [TypeParam::new("T")];

        // App(Array, [T]) unify App(Array, [String])
        let actual = Type::App(
            Box::new(Type::Array(Box::new(Type::Unknown))),
            vec![Type::String],
        );
        let pattern = Type::App(
            Box::new(Type::Array(Box::new(Type::Unknown))),
            vec![Type::Var("T".to_string())],
        );

        assert!(unify(&actual, &pattern, &mut subst, &tp));
        assert_eq!(subst.get("T"), Some(&Type::String));
    }

    #[test]
    fn hkt_substitute_replaces_app_vars() {
        let mut subst = Substitution::new();
        subst.insert("F".to_string(), Type::Array(Box::new(Type::Unknown)));
        subst.insert("T".to_string(), Type::String);

        // App(F, [T]) → App(Array<Unknown>, [String])
        let ty = Type::App(
            Box::new(Type::Var("F".to_string())),
            vec![Type::Var("T".to_string())],
        );
        let result = substitute(&ty, &subst);
        assert_eq!(
            result,
            Type::App(
                Box::new(Type::Array(Box::new(Type::Unknown))),
                vec![Type::String],
            )
        );
    }

    #[test]
    fn bind_var_to_concrete() {
        let mut subst = Substitution::new();
        assert!(unify(
            &Type::String,
            &Type::Var("T".to_string()),
            &mut subst,
            &[TypeParam::new("T")],
        ));
        assert_eq!(subst.get("T"), Some(&Type::String));
    }

    #[test]
    fn consistent_rebinding_succeeds() {
        let mut subst = Substitution::new();
        let tp = [TypeParam::new("T")];
        assert!(unify(
            &Type::String,
            &Type::Var("T".to_string()),
            &mut subst,
            &tp
        ));
        assert!(unify(
            &Type::String,
            &Type::Var("T".to_string()),
            &mut subst,
            &tp
        ));
        assert_eq!(subst.get("T"), Some(&Type::String));
    }

    #[test]
    fn inconsistent_rebinding_fails() {
        let mut subst = Substitution::new();
        let tp = [TypeParam::new("T")];
        assert!(unify(
            &Type::String,
            &Type::Var("T".to_string()),
            &mut subst,
            &tp
        ));
        assert!(!unify(
            &Type::Number,
            &Type::Var("T".to_string()),
            &mut subst,
            &tp
        ));
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
            &[TypeParam::new("T")],
        ));
        assert!(subst.is_empty());
    }

    #[test]
    fn unknown_existing_allows_more_specific() {
        let mut subst = Substitution::new();
        let tp = [TypeParam::new("T")];
        assert!(unify(
            &Type::Unknown,
            &Type::Var("T".to_string()),
            &mut subst,
            &tp
        ));
        assert!(unify(
            &Type::String,
            &Type::Var("T".to_string()),
            &mut subst,
            &tp
        ));
        assert_eq!(subst.get("T"), Some(&Type::String));
    }

    #[test]
    fn nested_array_unify() {
        let mut subst = Substitution::new();
        assert!(unify(
            &Type::Array(Box::new(Type::String)),
            &Type::Array(Box::new(Type::Var("T".to_string()))),
            &mut subst,
            &[TypeParam::new("T")],
        ));
        assert_eq!(subst.get("T"), Some(&Type::String));
    }

    #[test]
    fn function_unify_binds_vars() {
        let mut subst = Substitution::new();
        let tp = [TypeParam::new("T")];

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
            &[TypeParam::new("T")],
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
        let tp = [TypeParam::new("T"), TypeParam::new("U")];

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
        let tp = [TypeParam::new("T"), TypeParam::new("U")];

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
        assert_eq!(
            substitute(&return_ty, &subst),
            Type::Array(Box::new(Type::Number))
        );
    }

    #[test]
    fn check_bounds_satisfied() {
        let mut subst = Substitution::new();
        let tp = [TypeParam::with_bounds(
            "T",
            vec![Type::Trait("Numeric".to_string())],
        )];
        let registry = TraitRegistry::default_registry();

        assert!(unify(
            &Type::Number,
            &Type::Var("T".to_string()),
            &mut subst,
            &tp,
        ));
        assert!(check_bounds(&subst, &tp, &registry));
    }

    #[test]
    fn check_bounds_violated() {
        let mut subst = Substitution::new();
        let tp = [TypeParam::with_bounds(
            "T",
            vec![Type::Trait("Numeric".to_string())],
        )];
        let registry = TraitRegistry::default_registry();

        assert!(unify(
            &Type::String,
            &Type::Var("T".to_string()),
            &mut subst,
            &tp,
        ));
        assert!(!check_bounds(&subst, &tp, &registry));
    }

    #[test]
    fn check_bounds_empty_bounds_always_passes() {
        let mut subst = Substitution::new();
        let tp = [TypeParam::new("T")];
        let registry = TraitRegistry::default_registry();

        assert!(unify(
            &Type::String,
            &Type::Var("T".to_string()),
            &mut subst,
            &tp,
        ));
        assert!(check_bounds(&subst, &tp, &registry));
    }

    #[test]
    fn covariant_allows_compatible_rebinding() {
        let mut subst = Substitution::new();
        let tp = [TypeParam::new("T").with_variance(Variance::Covariant)];

        // First binding: T = String
        assert!(unify(
            &Type::String,
            &Type::Var("T".to_string()),
            &mut subst,
            &tp,
        ));
        // Second binding: T = String again — consistent
        assert!(unify(
            &Type::String,
            &Type::Var("T".to_string()),
            &mut subst,
            &tp,
        ));
    }

    #[test]
    fn invariant_rejects_different_rebinding() {
        let mut subst = Substitution::new();
        let tp = [TypeParam::new("T").with_variance(Variance::Invariant)];

        assert!(unify(
            &Type::String,
            &Type::Var("T".to_string()),
            &mut subst,
            &tp,
        ));
        // Invariant: different type must fail
        assert!(!unify(
            &Type::Number,
            &Type::Var("T".to_string()),
            &mut subst,
            &tp,
        ));
    }

    #[test]
    fn bivariant_always_consistent() {
        let mut subst = Substitution::new();
        let tp = [TypeParam::new("T").with_variance(Variance::Bivariant)];

        assert!(unify(
            &Type::String,
            &Type::Var("T".to_string()),
            &mut subst,
            &tp,
        ));
        // Bivariant: any type is consistent
        assert!(unify(
            &Type::Number,
            &Type::Var("T".to_string()),
            &mut subst,
            &tp,
        ));
    }
}