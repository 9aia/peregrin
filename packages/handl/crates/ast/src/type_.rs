/// Factory for constructing type nodes.
pub trait TypeFactory<T> {
    fn primitive<S>(&self, name: S) -> T
    where
        S: Into<String>;

    fn array(&self, element: T) -> T;

    fn object(&self, fields: Vec<(String, T)>) -> T;

    fn function(&self, params: Vec<T>, ret: T) -> T;
}
