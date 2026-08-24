/// Factory for constructing statement nodes.
pub trait StmtFactory<S, E> {
    fn expression(&self, expr: E) -> S;

    fn let_<N>(&self, name: N, value: E) -> S
    where
        N: Into<String>;

    fn block(&self, stmts: Vec<S>) -> S;
}
