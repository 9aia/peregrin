use ac_ast_common::ExprFactory;

/// An expression in the AC language.
///
/// Expressions form a tree that the VM evaluates to produce a
/// value. Five kinds:
///
/// - [`Expr::Value`] — a literal value.
/// - [`Expr::Call`] — an invocation of a host-provided function.
/// - [`Expr::Handle`] — a handler expression that intercepts operations.
/// - [`Expr::Array`] — an array literal with element expressions.
/// - [`Expr::Object`] — an object literal with key-value expression pairs.
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
        type_args: Vec<String>,
    },

    /// A handler expression — intercepts operations performed in `body`.
    Handle {
        /// The computation to run under this handler.
        body: Box<Expr<V>>,

        /// Handler clauses, checked in order.
        handlers: Vec<HandlerClause<V>>,
    },

    /// An array literal — `[expr, expr, ...]`.
    Array(Vec<Expr<V>>),

    /// An object literal — `{key: expr, key: expr, ...}`.
    Object(Vec<(String, Expr<V>)>),
}

/// A single handler clause in a [`Expr::Handle`] expression.
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

    /// Creates an array literal expression.
    pub fn array(elements: Vec<Expr<V>>) -> Self {
        Expr::Array(elements)
    }

    /// Creates an object literal expression.
    pub fn object(entries: Vec<(String, Expr<V>)>) -> Self {
        Expr::Object(entries)
    }
}

/// The canonical AC expression factory.
///
/// Implements `ExprFactory<Expr<V>, V>` so lexers, parsers, and embedded
/// consumers can build `Expr` trees through the generic factory interface.
#[derive(Debug, Default, Clone, Copy)]
pub struct CanonicalExprFactory;

impl<V> ExprFactory<Expr<V>, V> for CanonicalExprFactory {
    fn value(&self, value: V) -> Expr<V> {
        Expr::value(value)
    }

    fn call<S>(&self, name: S, args: Vec<Expr<V>>) -> Expr<V>
    where
        S: Into<String>,
    {
        Expr::call(name, args)
    }

    fn array(&self, elements: Vec<Expr<V>>) -> Expr<V> {
        Expr::array(elements)
    }

    fn object(&self, entries: Vec<(String, Expr<V>)>) -> Expr<V> {
        Expr::object(entries)
    }
}
