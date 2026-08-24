use std::marker::PhantomData;

use crate::NodeMetadata;

/// A generic AST node.
///
/// Parameterized by:
/// - `K` — the language-specific kind enum (expression, statement, type, ...).
/// - `V` — the value representation used by literals.
/// - `M` — the metadata attached to this node (default `()`).
///
/// The runtime can keep `M = ()` so the node is the same size as `K`.
/// Tools that need spans or attributes choose `M = Span`, `M = Attrs<A>`,
/// or a composed `Both<...>`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Node<K, V, M: NodeMetadata = ()> {
    pub kind: K,
    pub metadata: M,
    pub _value: PhantomData<V>,
}

impl<K, V> Node<K, V, ()> {
    /// Creates a node with no metadata.
    pub fn new(kind: K) -> Self {
        Self {
            kind,
            metadata: (),
            _value: PhantomData,
        }
    }
}

impl<K, V, M: NodeMetadata> Node<K, V, M> {
    /// Replaces the node's metadata, changing the `M` type parameter.
    pub fn with_metadata<N: NodeMetadata>(self, metadata: N) -> Node<K, V, N> {
        Node {
            kind: self.kind,
            metadata,
            _value: PhantomData,
        }
    }

    /// Maps the kind while preserving metadata and the value type.
    pub fn map_kind<F, K2>(self, f: F) -> Node<K2, V, M>
    where
        F: FnOnce(K) -> K2,
    {
        Node {
            kind: f(self.kind),
            metadata: self.metadata,
            _value: PhantomData,
        }
    }
}