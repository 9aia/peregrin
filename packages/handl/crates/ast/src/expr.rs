/// Factory for constructing expression nodes.
///
/// A `Lexer`/`Parser` or an embedded consumer can implement a custom
/// factory to attach its own metadata while still returning a standard
/// `Node<ExprKind, V, M>` tree.
pub trait ExprFactory<E, V> {
    fn value(&self, value: V) -> E;

    fn call<S>(&self, name: S, args: Vec<E>) -> E
    where
        S: Into<String>;

    fn apply(&self, target: E, args: Vec<E>) -> E;

    fn array(&self, elements: Vec<E>) -> E;

    fn object(&self, entries: Vec<(String, E)>) -> E;

    fn index(&self, target: E, index: E) -> E;
}
