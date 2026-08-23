use std::fmt::Debug;

/// Metadata attached to an AST node.
///
/// `()` is the zero-sized "no metadata" instance. Concrete metadata types
/// (`Span`, `Attrs`, `Both`) implement this trait so they can be plugged
/// into `Node<K, V, M>` without changing the node shape.
pub trait NodeMetadata: Default + Clone + Debug {}

impl NodeMetadata for () {}

/// A source span: byte offsets `[start, end)`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl NodeMetadata for Span {}

/// Arbitrary attributes attached to a node.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attrs<A: Clone + Default + Debug>(pub A);

impl<A: Clone + Default + Debug> NodeMetadata for Attrs<A> {}

/// Composed metadata: two metadata values side by side.
///
/// Use `Both` to layer `Span` with `Attrs`, two spans, or any other
/// combination without writing a new struct.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Both<M1: NodeMetadata, M2: NodeMetadata> {
    pub first: M1,
    pub second: M2,
}

impl<M1: NodeMetadata, M2: NodeMetadata> NodeMetadata for Both<M1, M2> {}
