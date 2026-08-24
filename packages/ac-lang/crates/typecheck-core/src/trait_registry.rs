use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::Type;

/// A registry mapping trait names to the concrete types that implement them.
///
/// Used during generic call instantiation to verify that a type variable's
/// bounds are satisfied by the concrete type it binds to.
///
/// # Built-in traits
///
/// The [`TraitRegistry::default`] registry includes:
/// - `Numeric` — `Number`
/// - `Textual` — `String`
/// - `Boolean` — `Boolean`
/// - `Nullable` — `Null`
/// - `Comparable` — `String`, `Number`, `Boolean`
/// - `Serializable` — all types (everything in AC is JSON-serializable)
///
/// Hosts can register custom traits via [`TraitRegistry::register`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitRegistry {
    traits: BTreeMap<String, Vec<Type>>,
}

impl TraitRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        TraitRegistry {
            traits: BTreeMap::new(),
        }
    }

    /// Create a registry with built-in traits.
    pub fn default_registry() -> Self {
        let mut registry = TraitRegistry::new();

        registry.register("Numeric", Type::Number);
        registry.register("Textual", Type::String);
        registry.register("Boolean", Type::Boolean);
        registry.register("Nullable", Type::Null);

        registry.register("Comparable", Type::String);
        registry.register("Comparable", Type::Number);
        registry.register("Comparable", Type::Boolean);

        registry.register("Serializable", Type::String);
        registry.register("Serializable", Type::Number);
        registry.register("Serializable", Type::Boolean);
        registry.register("Serializable", Type::Null);
        registry.register("Serializable", Type::Array(Box::new(Type::Unknown)));
        registry.register("Serializable", Type::Object(BTreeMap::new()));

        registry
    }

    /// Return a reference to the static default registry.
    ///
    /// Uses [`OnceLock`] to initialize the registry once and reuse it
    /// across all calls, eliminating per-call allocation.
    pub fn default_registry_ref() -> &'static TraitRegistry {
        static DEFAULT: OnceLock<TraitRegistry> = OnceLock::new();
        DEFAULT.get_or_init(Self::default_registry)
    }

    /// Register a type as implementing a trait.
    pub fn register(&mut self, trait_name: impl Into<String>, ty: Type) {
        self.traits.entry(trait_name.into()).or_default().push(ty);
    }

    /// Check if a type implements a named trait.
    ///
    /// Returns `true` if the type is registered as implementing the trait,
    /// or if the trait is not in the registry (unknown traits are treated
    /// as satisfied — graceful degradation).
    pub fn implements(&self, trait_name: &str, ty: &Type) -> bool {
        match self.traits.get(trait_name) {
            None => true,
            Some(implementors) => {
                if implementors.contains(ty) {
                    return true;
                }
                match ty {
                    Type::Unknown | Type::Never => true,
                    Type::Array(item) => {
                        implementors.contains(&Type::Array(Box::new(Type::Unknown)))
                            && self.implements(trait_name, item)
                    }
                    Type::Object(props) => {
                        implementors.contains(&Type::Object(BTreeMap::new()))
                            && props.values().all(|v| self.implements(trait_name, v))
                    }
                    _ => false,
                }
            }
        }
    }

    /// Check if a concrete type satisfies all given bounds.
    ///
    /// Each bound must be a `Type::Trait(name)` — other bound types are
    /// treated as exact-type requirements (the concrete type must match).
    pub fn check_bounds(&self, ty: &Type, bounds: &[Type]) -> bool {
        bounds.iter().all(|bound| match bound {
            Type::Trait(trait_name) => self.implements(trait_name, ty),
            Type::Unknown | Type::Never => true,
            other => other == ty,
        })
    }
}

impl Default for TraitRegistry {
    fn default() -> Self {
        Self::default_registry()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_registry_has_numeric() {
        let registry = TraitRegistry::default_registry();
        assert!(registry.implements("Numeric", &Type::Number));
        assert!(!registry.implements("Numeric", &Type::String));
    }

    #[test]
    fn default_registry_has_comparable() {
        let registry = TraitRegistry::default_registry();
        assert!(registry.implements("Comparable", &Type::String));
        assert!(registry.implements("Comparable", &Type::Number));
        assert!(registry.implements("Comparable", &Type::Boolean));
        assert!(!registry.implements("Comparable", &Type::Null));
    }

    #[test]
    fn unknown_trait_is_satisfied() {
        let registry = TraitRegistry::new();
        assert!(registry.implements("Nonexistent", &Type::String));
    }

    #[test]
    fn unknown_type_satisfies_any_bound() {
        let registry = TraitRegistry::default_registry();
        assert!(registry.implements("Numeric", &Type::Unknown));
    }

    #[test]
    fn check_bounds_with_trait_bound() {
        let registry = TraitRegistry::default_registry();
        let bounds = vec![Type::Trait("Numeric".to_string())];
        assert!(registry.check_bounds(&Type::Number, &bounds));
        assert!(!registry.check_bounds(&Type::String, &bounds));
    }

    #[test]
    fn check_bounds_with_no_bounds() {
        let registry = TraitRegistry::default_registry();
        assert!(registry.check_bounds(&Type::String, &[]));
    }

    #[test]
    fn check_bounds_with_exact_type() {
        let registry = TraitRegistry::default_registry();
        let bounds = vec![Type::Number];
        assert!(registry.check_bounds(&Type::Number, &bounds));
        assert!(!registry.check_bounds(&Type::String, &bounds));
    }

    #[test]
    fn custom_trait_registration() {
        let mut registry = TraitRegistry::new();
        registry.register("MyTrait", Type::String);
        registry.register("MyTrait", Type::Number);
        assert!(registry.implements("MyTrait", &Type::String));
        assert!(registry.implements("MyTrait", &Type::Number));
        assert!(!registry.implements("MyTrait", &Type::Boolean));
    }

    #[test]
    fn serializable_includes_array() {
        let registry = TraitRegistry::default_registry();
        assert!(registry.implements("Serializable", &Type::Array(Box::new(Type::String))));
    }
}
