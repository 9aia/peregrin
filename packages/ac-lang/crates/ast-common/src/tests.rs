use crate::{Attrs, Both, Node, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
enum MockKind {
    Value(i32),
    Call {
        name: String,
        args: Vec<Node<MockKind, i32>>}}

#[test]
fn node_with_no_metadata_is_zero_cost() {
    let node: Node<MockKind, i32> = Node::new(MockKind::Value(42));
    assert_eq!(node.metadata, ());
    assert_eq!(node.kind, MockKind::Value(42));
}

#[test]
fn node_can_carry_span() {
    let node: Node<MockKind, i32, Span> =
        Node::new(MockKind::Value(42)).with_metadata(Span { start: 10, end: 12 });
    assert_eq!(node.metadata, Span { start: 10, end: 12 });
}

#[test]
fn node_can_carry_composed_metadata() {
    let span = Span { start: 0, end: 5 };
    let attrs = Attrs("annotation".to_string());
    let meta = Both {
        first: span,
        second: attrs};

    let node: Node<MockKind, i32, Both<Span, Attrs<String>>> =
        Node::new(MockKind::Value(42)).with_metadata(meta);

    assert_eq!(node.metadata.first, span);
    assert_eq!(node.metadata.second, Attrs("annotation".to_string()));
}

#[test]
fn node_map_kind_preserves_metadata() {
    let node: Node<MockKind, i32, Span> =
        Node::new(MockKind::Value(42)).with_metadata(Span { start: 1, end: 2 });
    let mapped = node.map_kind(|k| match k {
        MockKind::Value(v) => MockKind::Call {
            name: "wrapped".to_string(),
            args: vec![Node::new(MockKind::Value(v))]},
        other => other});

    assert_eq!(mapped.metadata, Span { start: 1, end: 2 });
    match mapped.kind {
        MockKind::Call { name, .. } => assert_eq!(name, "wrapped"),
        _ => panic!("expected Call")}
}

#[test]
fn unit_metadata_is_zero_sized() {
    assert_eq!(std::mem::size_of::<()>(), 0);
}
