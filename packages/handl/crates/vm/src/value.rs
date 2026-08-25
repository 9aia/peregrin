use std::fmt::Debug;

/// A value that the handl VM can operate on.
pub trait Value: Clone + Debug + PartialEq + Send + Sync + 'static {}

impl<T> Value for T where T: Clone + Debug + PartialEq + Send + Sync + 'static {}
