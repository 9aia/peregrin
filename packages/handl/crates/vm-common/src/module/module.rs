use handl_vm::{AsyncFunction, Function, GenericFunction, LazyFunction, Operation};
use handl_vm_values_json::NonFunctionValue;

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
///     fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
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
    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>>;

    /// Resolve a function name to a lazy callable.
    ///
    /// Returns `None` by default. Override to provide functions that
    /// need lazy (short-circuit) argument evaluation.
    fn lazy_function(&self, _name: &str) -> Option<&dyn LazyFunction<NonFunctionValue>> {
        None
    }

    /// Resolve a function name to a [`GenericFunction`] that receives
    /// resolved type arguments.
    ///
    /// Returns `None` by default. Override to provide generic functions
    /// that need to inspect type arguments at runtime. The VM calls
    /// this when a call carries non-empty `type_args`.
    fn generic_function(&self, _name: &str) -> Option<&dyn GenericFunction<NonFunctionValue>> {
        None
    }

    /// Resolve a function name to an [`AsyncFunction`].
    ///
    /// Returns `None` by default. Override to provide async functions
    /// (network I/O, timers, etc.) for this module.
    fn async_function(&self, _name: &str) -> Option<&dyn AsyncFunction<NonFunctionValue>> {
        None
    }

    /// Resolve a function name to a typed [`Operation`].
    ///
    /// Returns `None` by default. Override to provide algebraic-effect
    /// operations for this module.
    fn operation(&self, _name: &str) -> Option<&dyn Operation<NonFunctionValue>> {
        None
    }
}
