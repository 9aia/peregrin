use std::collections::HashMap;

use crate::Type;

/// A type environment — maps field names to their declared types.
///
/// Supports scope chaining for inheritance: a child scope inherits
/// all bindings from its parent and can override individual fields.
/// Lookups walk the chain from child to parent.
pub struct TypeEnv {
    /// Field name → type in this scope.
    bindings: HashMap<String, Type>,
    /// Parent scope, if any.
    parent: Option<Box<TypeEnv>>}

impl TypeEnv {
    /// Create an empty environment with no parent.
    pub fn new() -> Self {
        TypeEnv {
            bindings: HashMap::new(),
            parent: None}
    }

    /// Create a child scope that inherits from `parent`.
    pub fn child(parent: TypeEnv) -> Self {
        TypeEnv {
            bindings: HashMap::new(),
            parent: Some(Box::new(parent))}
    }

    /// Insert or update a binding in this scope.
    pub fn insert(&mut self, name: impl Into<String>, ty: Type) {
        self.bindings.insert(name.into(), ty);
    }

    /// Insert multiple bindings from `(name, type)` pairs.
    pub fn extend(&mut self, entries: impl IntoIterator<Item = (String, Type)>) {
        for (name, ty) in entries {
            self.bindings.insert(name, ty);
        }
    }

    /// Look up a field's type, walking the scope chain.
    pub fn lookup(&self, name: &str) -> Option<&Type> {
        if let Some(ty) = self.bindings.get(name) {
            return Some(ty);
        }
        self.parent.as_deref().and_then(|p| p.lookup(name))
    }

    /// Returns `true` if the field exists in this scope or any parent.
    pub fn contains(&self, name: &str) -> bool {
        self.lookup(name).is_some()
    }

    /// Iterate over all bindings in this scope (not including parents).
    pub fn local_bindings(&self) -> impl Iterator<Item = (&String, &Type)> {
        self.bindings.iter()
    }

    /// Collect all bindings from this scope and all parents into a map.
    /// Child bindings shadow parent bindings with the same name.
    ///
    /// **Warning**: This eagerly clones the entire scope chain. Prefer
    /// [`all_bindings_iter`] when you only need to iterate.
    pub fn all_bindings(&self) -> HashMap<String, Type> {
        let mut result = HashMap::new();
        if let Some(ref parent) = self.parent {
            result.extend(parent.all_bindings());
        }
        for (name, ty) in &self.bindings {
            result.insert(name.clone(), ty.clone());
        }
        result
    }

    /// Iterate over all bindings from this scope and all parents.
    ///
    /// Child bindings shadow parent bindings with the same name —
    /// only the innermost binding for a given name is yielded.
    ///
    /// Unlike [`all_bindings`], this does not clone any types.
    pub fn all_bindings_iter(&self) -> impl Iterator<Item = (&String, &Type)> {
        let mut seen = std::collections::HashSet::new();
        let mut current = Some(self);
        std::iter::from_fn(move || {
            while let Some(env) = current {
                for (name, ty) in &env.bindings {
                    if seen.insert(name.as_str()) {
                        return Some((name, ty));
                    }
                }
                current = env.parent.as_deref();
            }
            None
        })
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}
