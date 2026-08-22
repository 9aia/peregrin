/// An expression in the language.
///
/// Expressions form a tree that the VM evaluates to produce a
/// value. Three kinds:
///
/// - [`Expr::Value`] — a literal value.
/// - [`Expr::Call`] — an invocation of a host-provided function.
/// - [`Expr::Handle`] — a handler expression that intercepts operations.
///
/// `Expr` is generic over the value type `V`, keeping `ac-ast`
/// dependency-free. The concrete value type (`NonFunctionValue`) is
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

        /// Resolved type arguments for generic function calls.
        ///
        /// Populated by the typechecker during monomorphization. Empty for
        /// monomorphic calls. Each entry is a string representation of the
        /// concrete type (e.g. `"string"`, `"number"`, `"array<string>"`).
        ///
        /// Host functions can inspect this via the `GenericFunction` trait
        /// to implement type-aware behavior at runtime.
        type_args: Vec<String>,
    },

    /// A handler expression — intercepts operations performed in `body`.
    ///
    /// When an operation signals during evaluation of `body`, the VM
    /// checks each handler clause in order. If a clause's `op_name`
    /// matches the operation name, the handler body is evaluated with
    /// the operation's arguments bound to `params` and a resumable
    /// continuation bound to `continuation_name`.
    ///
    /// If no clause matches, the operation propagates to the next
    /// enclosing handler (or to the caller if there is none).
    Handle {
        /// The computation to run under this handler.
        body: Box<Expr<V>>,

        /// Handler clauses, checked in order.
        handlers: Vec<HandlerClause<V>>,
    },
}

/// A single handler clause in a [`Expr::Handle`] expression.
///
/// Matches operations by name. When an operation with `op_name` signals,
/// the handler `body` is evaluated with:
/// - `params` bound to the operation's evaluated arguments
/// - `continuation_name` bound to a resumable continuation value
#[derive(Debug, Clone, PartialEq)]
pub struct HandlerClause<V> {
    /// The operation name to match (e.g. `"fetch"`).
    pub op_name: String,

    /// Parameter names bound to the operation's arguments.
    pub params: Vec<String>,

    /// The name bound to the captured continuation.
    pub continuation_name: String,

    /// The handler body expression.
    pub body: Expr<V>,
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
            type_args: Vec::new(),
        }
    }

    /// Creates a call expression with resolved type arguments.
    ///
    /// Used by the typechecker during monomorphization to attach
    /// concrete type information to generic call sites.
    pub fn call_with_type_args(
        name: impl Into<String>,
        args: Vec<Expr<V>>,
        type_args: Vec<String>,
    ) -> Self {
        Expr::Call {
            name: name.into(),
            args,
            type_args,
        }
    }

    /// Creates a handle expression.
    pub fn handle(body: Expr<V>, handlers: Vec<HandlerClause<V>>) -> Self {
        Expr::Handle {
            body: Box::new(body),
            handlers,
        }
    }
}
