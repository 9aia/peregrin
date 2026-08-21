use ac_vm::Function;

/// A named group of functions.
///
/// A module provides a namespace for host functions. The module name
/// becomes the first segment of a qualified function name — e.g. a
/// module named `"math"` providing a function `"abs"` is callable as
/// `math.abs(x)`.
///
/// The `Module` trait is the runtime-side analogue of the language's
/// namespace concept: named scopes for path lookups. Which modules
/// exist is a runtime host decision, not a language-level feature.
///
/// ## Implementing a module
///
/// ```ignore
/// struct MathModule;
///
/// impl Module for MathModule {
///     fn name(&self) -> &str { "math" }
///     fn function(&self, name: &str) -> Option<&dyn Function> {
///         match name {
///             "abs" => Some(&Abs),
///             _ => None,
///         }
///     }
/// }
/// ```
pub trait Module: Send + Sync {
    /// The module name — the namespace prefix for qualified calls.
    fn name(&self) -> &str;

    /// Resolve a function name (without the module prefix) to a callable.
    ///
    /// Returns `None` if the name is not recognized in this module.
    fn function(&self, name: &str) -> Option<&dyn Function>;
}
