use crate::Error;
use ac_runtime::RuntimeValue;
use indexmap::IndexMap;

/// The execution context for a VM evaluation.
///
/// A context maintains a stack of variable scopes. Each scope is
/// a map of name-value pairs with O(1) lookup. Variable lookup
/// searches from the innermost scope outward, returning the first
/// match.
///
/// Scopes are pushed and popped to model function call boundaries.
/// The bottommost scope is the global scope and is never popped.
pub struct Context {
    scopes: Vec<IndexMap<String, RuntimeValue>>,
}

impl Context {
    /// Creates a new context with a single empty global scope.
    pub fn new() -> Self {
        Context {
            scopes: vec![IndexMap::new()],
        }
    }

    /// Pushes a new empty scope onto the scope stack.
    pub fn push_scope(&mut self) {
        self.scopes.push(IndexMap::new());
    }

    /// Pops the topmost scope from the stack.
    ///
    /// Returns the popped scope's entries, or `None` if only the
    /// global scope remains (the global scope cannot be popped).
    pub fn pop_scope(&mut self) -> Option<IndexMap<String, RuntimeValue>> {
        if self.scopes.len() > 1 {
            self.scopes.pop()
        } else {
            None
        }
    }

    /// Returns the current scope depth.
    ///
    /// The global scope is depth 1.
    pub fn depth(&self) -> usize {
        self.scopes.len()
    }

    /// Defines a variable in the current (topmost) scope.
    ///
    /// If a variable with the same name exists in the current
    /// scope, it is overwritten.
    pub fn define(&mut self, name: impl Into<String>, value: RuntimeValue) {
        self.scopes.last_mut().unwrap().insert(name.into(), value);
    }

    /// Looks up a variable by name, searching from the innermost
    /// scope outward.
    ///
    /// Returns a clone of the value if found, or
    /// [`Error::UnknownVariable`] if not.
    pub fn get(&self, name: &str) -> Result<RuntimeValue, Error> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Ok(value.clone());
            }
        }
        Err(Error::UnknownVariable(name.to_string()))
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
