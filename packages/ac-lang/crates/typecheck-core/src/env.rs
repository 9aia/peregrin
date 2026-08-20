use std::collections::BTreeMap;

use crate::Type;

/// A type environment — maps field names to their declared types.
///
/// Supports scope chaining for inheritance: a child scope inherits
/// all bindings from its parent and can override individual fields.
/// Lookups walk the chain from child to parent.
pub struct TypeEnv {
    /// Field name → type in this scope.
    bindings: BTreeMap<String, Type>,
    /// Parent scope, if any.
    parent: Option<Box<TypeEnv>>,
}

impl TypeEnv {
    /// Create an empty environment with no parent.
    pub fn new() -> Self {
        TypeEnv {
            bindings: BTreeMap::new(),
            parent: None,
        }
    }

    /// Create a child scope that inherits from `parent`.
    pub fn child(parent: TypeEnv) -> Self {
        TypeEnv {
            bindings: BTreeMap::new(),
            parent: Some(Box::new(parent)),
        }
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
    pub fn all_bindings(&self) -> BTreeMap<String, Type> {
        let mut result = BTreeMap::new();
        if let Some(ref parent) = self.parent {
            result.extend(parent.all_bindings());
        }
        for (name, ty) in &self.bindings {
            result.insert(name.clone(), ty.clone());
        }
        result
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}
