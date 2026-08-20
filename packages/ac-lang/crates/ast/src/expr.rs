/// An expression in the language.
///
/// Expressions form a tree that the VM evaluates to produce a
/// value. Two kinds:
///
/// - [`Expr::Value`] — a literal value.
/// - [`Expr::Call`] — an invocation of a host-provided function.
///
/// `Expr` is generic over the value type `V`, keeping `ac-ast`
/// dependency-free. The concrete value type (`RuntimeValue`) is
/// chosen by the consumer — typically the VM.
///
/// The expression tree is the sole description of computation.
/// The core does not define how expressions are evaluated — that
/// is the VM's responsibility.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr<V> {
    /// A literal value.
    Value(V),

    /// A call to a host-provided function.
    Call {
        /// The name of the function to invoke.
        name: String,

        /// Arguments to pass to the function, in order.
        args: Vec<Expr<V>>,
    },
}

impl<V> Expr<V> {
    /// Creates a value expression.
    pub fn value(value: V) -> Self {
        Expr::Value(value)
    }

    /// Creates a call expression.
    pub fn call(name: impl Into<String>, args: Vec<Expr<V>>) -> Self {
        Expr::Call {
            name: name.into(),
            args,
        }
    }
}
