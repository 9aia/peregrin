use ac_runtime::RuntimeValue;

use crate::Error;

/// A callback that evaluates an expression string with variable bindings.
///
/// This trait bridges the VM's [`Function`] trait to the higher-level
/// expression evaluator (parser + resolver). It allows host functions
/// like `fp.map` to evaluate function bodies — expression strings that
/// reference bound parameters.
///
/// The implementor is responsible for parsing the expression string,
/// resolving variable references, and dispatching nested function calls.
///
/// ## Implementing
///
/// Typically implemented in the compiler layer, where the dialect parser
/// and function resolver are available:
///
/// ```ignore
/// impl Eval for ExprEval {
///     fn eval_expr(
///         &self,
///         expr: &str,
///         vars: &[(String, RuntimeValue)],
///     ) -> Result<RuntimeValue, Error> {
///         // Parse expr, bind vars, evaluate, return result
///     }
/// }
/// ```
pub trait Eval {
    /// Evaluate an expression string with the given variable bindings.
    ///
    /// # Arguments
    ///
    /// - `expr` — the expression string (e.g. `"mul(x, 2)"`).
    /// - `vars` — `(name, value)` pairs to bind before evaluation.
    ///
    /// # Returns
    ///
    /// The evaluated [`RuntimeValue`], or an [`Error`] if evaluation failed.
    fn eval_expr(
        &self,
        expr: &str,
        vars: &[(String, RuntimeValue)],
    ) -> Result<RuntimeValue, Error>;
}
