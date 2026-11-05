//! Per-message rewind — the `rewind` selector the node's
//! `POST /api/sessions/{id}/rewind` takes next to `{confirm}`.
//! `nodeId` keeps that transcript node and everything before it (the
//! per-message target); `checkpoint` rewinds to a recorded snapshot
//! ref. A missing selector capability surfaces on the summary row as
//! `canRewind`, so callers only shape the request — the store itself
//! decides whether the cut lands.

use serde::{Deserialize, Serialize};

/// The `rewind` selector — serializes into the wire field names the
/// node expects (`{"nodeId": …}` / `{"checkpoint": …}`), so it merges
/// into the POST body verbatim next to `{"confirm": true}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RewindSelector {
    /// Keep this transcript node and everything before it — the
    /// per-message target the hover strip passes.
    NodeId(i64),
    /// Keep up to the last node recorded at/before this checkpoint ref.
    Checkpoint(String),
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn selector_serializes_to_wire_field_names() {
        assert_eq!(
            serde_json::to_value(RewindSelector::NodeId(7)).unwrap(),
            serde_json::json!({ "nodeId": 7 })
        );
        assert_eq!(
            serde_json::to_value(RewindSelector::Checkpoint("cp-1".into())).unwrap(),
            serde_json::json!({ "checkpoint": "cp-1" })
        );
        // And merges into the request body next to `confirm`.
        let mut body = serde_json::json!({ "confirm": true });
        body.as_object_mut().unwrap().extend(
            serde_json::to_value(RewindSelector::NodeId(3))
                .unwrap()
                .as_object()
                .unwrap()
                .clone(),
        );
        assert_eq!(body, serde_json::json!({ "confirm": true, "nodeId": 3 }));
        // Tolerant decode — server fns deserialize the same shape.
        let parsed: RewindSelector =
            serde_json::from_value(serde_json::json!({ "nodeId": 9 })).unwrap();
        assert_eq!(parsed, RewindSelector::NodeId(9));
    }
}
