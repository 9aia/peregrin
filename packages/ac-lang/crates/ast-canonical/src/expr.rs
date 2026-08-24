use ac_ast_common::ExprFactory;

/// A pattern used in binding positions: left-hand side of `=` or
/// function parameters.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// A simple identifier binding, e.g. `x`.
    Identifier(String),
    /// An array destructuring pattern, e.g. `[a, b]` or `[a, ...rest]`.
    Array(Vec<Pattern>),
    /// An object destructuring pattern, e.g. `{x, y}` or `{x: a, y: b}`.
    ///
    /// The first `String` is the object key; the nested `Pattern`
    /// is the binding for that key's value.
    Object(Vec<(String, Pattern)>),
    /// A rest pattern, e.g. `...rest`, capturing the remainder of
    /// an array or object.
    Rest(String),
}

impl Pattern {
    /// Returns the set of variable names bound by this pattern, in
    /// a deterministic order (left-to-right, depth-first).
    pub fn bound_names(&self) -> Vec<String> {
        match self {
            Pattern::Identifier(name) => vec![name.clone()],
            Pattern::Array(elements) => elements.iter().flat_map(Pattern::bound_names).collect(),
            Pattern::Object(entries) => entries.iter().flat_map(|(_, p)| p.bound_names()).collect(),
            Pattern::Rest(name) => vec![name.clone()],
        }
    }
}

/// An expression in the AC language.
///
/// Expressions form a tree that the VM evaluates to produce a
/// value. Ten kinds:
///
/// - [`Expr::Value`] — a literal value.
/// - [`Expr::Var`] — a bare identifier value.
/// - [`Expr::Assign`] — a pattern binding.
/// - [`Expr::Function`] — a first-class function literal.
/// - [`Expr::Call`] — an invocation of a host-provided function.
/// - [`Expr::Apply`] — an invocation of an arbitrary function value.
/// - [`Expr::Handle`] — a handler expression that intercepts operations.
/// - [`Expr::Array`] — an array literal with element expressions.
/// - [`Expr::Object`] — an object literal with key-value expression pairs.
/// - [`Expr::Index`] — a postfix index: `arr[0]`, `obj["key"]`, `obj[varName]`.
/// - [`Expr::FieldAccess`] — a postfix field access: `a.b`, `obj.key`.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr<V> {
    /// A literal value.
    Value(V),

    /// A bare identifier value — a variable or a namespaced type like
    /// `type.number`.
    Var(String),

    /// An assignment — `pattern = value`.
    Assign {
        /// The pattern being bound.
        target: Pattern,

        /// The value to bind.
        value: Box<Expr<V>>,
    },

    /// A first-class function literal.
    Function {
        /// Generic type parameters, in declaration order.
        type_params: Vec<String>,

        /// Parameters, each with an optional type expression.
        params: Vec<(Pattern, Option<Box<Expr<V>>>)>,

        /// Optional return type expression.
        ret: Option<Box<Expr<V>>>,

        /// The original source text of the function body.
        body: String,
    },

    /// A call to a host-provided function.
    Call {
        /// The name of the function to invoke.
        name: String,

        /// Arguments to pass to the function, in order.
        args: Vec<Expr<V>>,

        /// Resolved type arguments for generic function calls.
        type_args: Vec<String>,
    },

    /// An invocation of an arbitrary expression that evaluates to a
    /// first-class function value.
    Apply {
        /// The expression to evaluate to a function value.
        target: Box<Expr<V>>,

        /// Arguments to pass to the function, in order.
        args: Vec<Expr<V>>,
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

    /// A postfix index — `arr[0]`, `obj["key"]`, `obj[varName]`.
    Index {
        /// The collection being indexed.
        target: Box<Expr<V>>,

        /// The index expression.
        index: Box<Expr<V>>,
    },

    /// A postfix field access — `a.b`, `obj.key`.
    FieldAccess {
        /// The expression being accessed.
        target: Box<Expr<V>>,

        /// The field name.
        field: String,
    },
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

    /// Creates an apply expression that calls an arbitrary function value.
    pub fn apply(target: Expr<V>, args: Vec<Expr<V>>) -> Self {
        Expr::Apply {
            target: Box::new(target),
            args,
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

    /// Creates a postfix index expression.
    pub fn index(target: Expr<V>, index: Expr<V>) -> Self {
        Expr::Index {
            target: Box::new(target),
            index: Box::new(index),
        }
    }

    /// Creates a postfix field access expression.
    pub fn field_access(target: Expr<V>, field: impl Into<String>) -> Self {
        Expr::FieldAccess {
            target: Box::new(target),
            field: field.into(),
        }
    }

    /// Creates a variable expression.
    pub fn var(name: impl Into<String>) -> Self {
        Expr::Var(name.into())
    }

    /// Creates an assignment expression.
    pub fn assign(target: Pattern, value: Expr<V>) -> Self {
        Expr::Assign {
            target,
            value: Box::new(value),
        }
    }

    /// Creates a first-class function expression.
    pub fn function(
        type_params: Vec<String>,
        params: Vec<(Pattern, Option<Box<Expr<V>>>)>,
        ret: Option<Box<Expr<V>>>,
        body: impl Into<String>,
    ) -> Self {
        Expr::Function {
            type_params,
            params,
            ret,
            body: body.into(),
        }
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

    fn apply(&self, target: Expr<V>, args: Vec<Expr<V>>) -> Expr<V> {
        Expr::apply(target, args)
    }

    fn array(&self, elements: Vec<Expr<V>>) -> Expr<V> {
        Expr::array(elements)
    }

    fn object(&self, entries: Vec<(String, Expr<V>)>) -> Expr<V> {
        Expr::object(entries)
    }

    fn index(&self, target: Expr<V>, index: Expr<V>) -> Expr<V> {
        Expr::index(target, index)
    }
}
