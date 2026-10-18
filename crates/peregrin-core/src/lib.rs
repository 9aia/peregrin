//! peregrin-core — the pure domain: events, campaigns, nodes, and the
//! graph model every other crate speaks. No I/O.

/// A domain event — the unit the bus carries and nodes react to.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Event {
    /// Event type tag — routing key for subscribers and node triggers.
    pub kind: String,
    /// JSON payload — validated by producers/consumers, opaque here.
    pub payload: serde_json::Value,
}

/// A node in a campaign graph — id + driver + params; edges come from
/// `on` triggers, not explicit links (port of the TS campaign model).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub id: String,
    pub driver: String,
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(default)]
    pub on: Vec<String>,
}

/// A campaign — a named graph of nodes plus its triggers.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Campaign {
    pub name: String,
    #[serde(default)]
    pub nodes: Vec<Node>,
}
