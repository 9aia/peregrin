/// A parsed handler clause — the result of parsing a single
/// `"op"(params) as k: body` clause from a handle expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedHandlerClause {
    /// The operation name to match (e.g. `"fetch"`).
    pub op_name: String,

    /// Parameter names bound to the operation's arguments.
    pub params: Vec<String>,

    /// The name bound to the captured continuation.
    pub continuation_name: String,

    /// The handler body expression string (unparsed).
    pub body: String,
}

/// Expression parsing operations for handl expressions.
///
/// These operations parse expression strings into their components —
/// detecting function calls, splitting arguments, and recognizing literals.
/// All literal parsing is dialect-configurable: a dialect that uses
/// `yes`/`no` for booleans or `nil` for null implements
/// `parse_bool_literal` and `is_null_literal` accordingly.
///
/// The value type system (see [LANG.md](../../docs/LANG.md) §3) defines the
/// _output_ types — boolean, null, floating-point number, string — not the
/// _input_ syntax. A dialect changes how literals are recognized; the values
/// they produce remain the same.
pub trait ExprParser: Send + Sync {
    /// Detect whether `s` contains a function call opening (e.g. `(` in canonical).
    /// Returns the byte position of the call-open token if present and not
    /// inside a string literal.
    fn find_call_open(&self, s: &str) -> Option<usize>;

    /// Split function arguments by the argument separator (`,` in canonical),
    /// respecting nested call-open/close tokens and string literals.
    fn split_args(&self, s: &str) -> Vec<String>;

    /// Check if `s` is a string literal and return its inner content.
    fn parse_string_literal(&self, s: &str) -> Option<String>;

    /// Check if `s` is a number literal and return its value.
    fn parse_number_literal(&self, s: &str) -> Option<f64>;

    /// Check if `s` is a boolean literal and return its value.
    fn parse_bool_literal(&self, s: &str) -> Option<bool>;

    /// Check if `s` is the null literal.
    fn is_null_literal(&self, s: &str) -> bool;

    /// Detect whether `s` starts with the `handle` keyword followed by `(`.
    /// Returns the byte position of the `(` after `handle` if present.
    ///
    /// The keyword must be a standalone word — `handleIt(` does not match.
    /// String literals are skipped.
    fn find_handle_open(&self, _s: &str) -> Option<usize> {
        None
    }

    /// Find the position of the block-open token (`{` in canonical) in `s`.
    /// Used to locate the handler clause block in a handle expression.
    /// Returns the byte position of the first `{` outside of string literals.
    fn find_block_open(&self, _s: &str) -> Option<usize> {
        None
    }

    /// Split handler clauses by the clause separator (`,` in canonical),
    /// respecting nested parentheses, braces, and string literals.
    fn split_handler_clauses(&self, s: &str) -> Vec<String> {
        let _ = s;
        Vec::new()
    }

    /// Parse a single handler clause string (e.g. `"fetch"(url) as k: k(42)`)
    /// into a [`ParsedHandlerClause`].
    fn parse_handler_clause(&self, s: &str) -> Option<ParsedHandlerClause> {
        let _ = s;
        None
    }
}
