#![allow(unknown_lints, clippy::assert_is_empty)]

//! `from_dict`/`to_dict` normalization tests — the typed-model half of
//! the TS `model.ts` port (poll nodes, self-trigger defaults, schema
//! compilation, campaign-level normalization).
#![allow(clippy::unwrap_used)]

use peregrin_core::{Campaign, EventSchema, MaxFires, Node, SupersedeMode, TriggerMode};
use serde_json::json;

#[test]
fn node_defaults_from_minimal_dict() {
    let node = Node::from_dict(&json!({"id": "n", "driver": "shell", "on": ["start"]}));
    assert_eq!(node.id, "n");
    assert_eq!(node.mode, TriggerMode::Any);
    assert_eq!(node.max_fires, MaxFires::Fires(1));
    assert!(node.emits.is_empty());
}

#[test]
fn scalar_fields_coerce_like_javascript() {
    let node = Node::from_dict(&json!({"id": 42, "driver": true}));
    assert_eq!(node.id, "42");
    assert_eq!(node.driver, "true");
}

#[test]
fn poll_node_gets_tick_trigger_and_interval_param() {
    let node = Node::from_dict(&json!({
        "id": "p",
        "driver": "polling",
        "poll": {"interval": 30, "until": ["done"]},
        "on": ["boot"],
    }));
    assert_eq!(node.on, ["boot", "p.tick"]);
    assert_eq!(node.emits, ["p.tick"]);
    assert_eq!(node.until, ["done"]);
    assert_eq!(node.params["interval"], 30);
    assert!(node.poll.is_some());
}

#[test]
fn poll_node_does_not_overwrite_local_interval() {
    let node = Node::from_dict(&json!({
        "id": "p",
        "poll": {"interval": 30},
        "params": {"interval": 5},
    }));
    assert_eq!(node.params["interval"], 5);
}

#[test]
fn self_trigger_drivers_default_max_fires_to_100() {
    let cli = Node::from_dict(&json!({"id": "c", "driver": "cli"}));
    assert_eq!(cli.max_fires, MaxFires::Fires(100));
    let polling = Node::from_dict(&json!({"id": "p", "driver": "polling"}));
    assert_eq!(polling.max_fires, MaxFires::Fires(100));
    // `params.mode: interval` polls continuously — default stays 1.
    let interval =
        Node::from_dict(&json!({"id": "p", "driver": "cli", "params": {"mode": "interval"}}));
    assert_eq!(interval.max_fires, MaxFires::Fires(1));
    let shell = Node::from_dict(&json!({"id": "s", "driver": "shell"}));
    assert_eq!(shell.max_fires, MaxFires::Fires(1));
}

#[test]
fn max_fires_parses_unlimited_and_integers() {
    let unlimited = Node::from_dict(&json!({"max_fires": "unlimited"}));
    assert_eq!(unlimited.max_fires, MaxFires::Unlimited);
    let float_integer = Node::from_dict(&json!({"max_fires": 3.0}));
    assert_eq!(float_integer.max_fires, MaxFires::Fires(3));
    let zero = Node::from_dict(&json!({"max_fires": 0}));
    assert_eq!(zero.max_fires, MaxFires::Fires(1));
}

#[test]
fn event_schema_compiles_fields_to_json_schema() {
    let schema = EventSchema::from_dict(&json!({
        "type": "deploy.done",
        "fields": {"stdout": "string", "code": "integer?"},
    }));
    assert_eq!(
        schema.compiled_schema().unwrap(),
        json!({
            "type": "object",
            "properties": {
                "stdout": {"type": "string"},
                "code": {"type": "integer"},
            },
            "required": ["stdout"],
        })
    );
    let empty = EventSchema::from_dict(&json!({"type": "x"}));
    assert!(empty.compiled_schema().is_none());
    let explicit = EventSchema::from_dict(&json!({
        "type": "x",
        "fields": {"a": "string"},
        "schema": {"type": "object"},
    }));
    assert_eq!(
        explicit.compiled_schema().unwrap(),
        json!({"type": "object"})
    );
}

#[test]
fn node_input_output_schemas_parse() {
    let node = Node::from_dict(&json!({
        "id": "n",
        "input": {"fields": {"a": "string"}},
        "output": {"done": {"fields": {"b": "integer"}}, "failed": "bare"},
    }));
    assert_eq!(node.input.as_ref().unwrap().kind, "");
    assert_eq!(node.output.len(), 2);
    assert_eq!(node.output[0].kind, "done");
    assert_eq!(node.output[1].kind, "failed");
}

#[test]
fn campaign_normalizes_functions_imports_and_python() {
    let campaign = Campaign::from_dict(&json!({
        "name": "w",
        "functions": {"deploy": {"driver": "shell"}, "empty": 5},
        "includes": ["a.yaml"],
        "import": ["lib.py", {"file": "mod.py", "as": "m"}, 7],
        "stop_on": ["workflow.done"],
        "strict": 1,
        "config": {"k": "v"},
    }))
    .unwrap();
    assert_eq!(campaign.functions.len(), 2);
    assert_eq!(campaign.functions[0].name, "deploy");
    assert_eq!(campaign.functions[1].name, "empty");
    assert_eq!(campaign.imports, ["a.yaml"]);
    assert_eq!(campaign.python_imports.len(), 2);
    assert_eq!(campaign.python_imports[1].file, "mod.py");
    assert_eq!(campaign.python_imports[1].alias, "m");
    assert_eq!(campaign.stop_on, ["workflow.done"]);
    assert!(campaign.strict);
    assert_eq!(campaign.config["k"], "v");
}

#[test]
fn campaign_functions_as_array_keep_dicts() {
    let campaign = Campaign::from_dict(&json!({
        "functions": [{"name": "f", "driver": "shell"}, "skip-me"],
    }))
    .unwrap();
    assert_eq!(campaign.functions.len(), 1);
    assert_eq!(campaign.functions[0].name, "f");
}

#[test]
fn campaign_rejects_non_array_nodes_and_events() {
    let err = Campaign::from_dict(&json!({"nodes": "nope"})).unwrap_err();
    assert_eq!(err.field, "nodes");
    let err = Campaign::from_dict(&json!({"events": 5})).unwrap_err();
    assert_eq!(err.field, "events");
}

// ---------------------------------------------------------------------------
// supersede: parsing (docs/reference/queue-supersede.md)
// ---------------------------------------------------------------------------

#[test]
fn campaign_parses_supersede_block() {
    let campaign = Campaign::from_dict(&json!({
        "supersede": {
            "key": "{repo}:{ref}",
            "mode": "fifo",
            "cancel_running": true,
        },
    }))
    .unwrap();
    let supersede = campaign.supersede.unwrap();
    assert_eq!(supersede.key, "{repo}:{ref}");
    assert_eq!(supersede.mode, SupersedeMode::Fifo);
    assert!(supersede.cancel_running);
}

#[test]
fn campaign_supersede_defaults_partial_block() {
    let campaign = Campaign::from_dict(&json!({
        "supersede": {"cancel_running": true},
    }))
    .unwrap();
    let supersede = campaign.supersede.unwrap();
    assert_eq!(supersede.key, "");
    assert_eq!(supersede.mode, SupersedeMode::LatestWins);
    // `supersede:` absent or null → no policy.
    assert!(Campaign::from_dict(&json!({})).unwrap().supersede.is_none());
    assert!(
        Campaign::from_dict(&json!({"supersede": null}))
            .unwrap()
            .supersede
            .is_none()
    );
}

#[test]
fn campaign_rejects_bad_supersede() {
    // Unknown `mode` — a closed enum, not a coerced string.
    let err = Campaign::from_dict(&json!({
        "supersede": {"mode": "newest-only"},
    }))
    .unwrap_err();
    assert_eq!(err.field, "supersede");
    // Unknown keys are denied (typo'd `cancel` would silently
    // misdeclare the policy otherwise).
    let err = Campaign::from_dict(&json!({
        "supersede": {"cancel": true},
    }))
    .unwrap_err();
    assert_eq!(err.field, "supersede");
    // A non-dict block fails the same channel.
    let err = Campaign::from_dict(&json!({"supersede": "latest-wins"})).unwrap_err();
    assert_eq!(err.field, "supersede");
}

#[test]
fn campaign_supersede_round_trips_to_dict() {
    let campaign = Campaign::from_dict(&json!({
        "supersede": {"key": "{repo}", "mode": "fifo"},
    }))
    .unwrap();
    let dict = campaign.to_dict();
    assert_eq!(dict["supersede"]["key"], "{repo}");
    assert_eq!(dict["supersede"]["mode"], "fifo");
    assert_eq!(dict["supersede"]["cancel_running"], false);
    let reparsed = Campaign::from_dict(&dict).unwrap();
    assert_eq!(reparsed.supersede, campaign.supersede);
    // No policy → no serialized key.
    assert!(Campaign::default().to_dict().get("supersede").is_none());
}

#[test]
fn campaign_merge_pulls_missing_events_functions_and_config() {
    let mut a = Campaign::from_dict(&json!({
        "events": [{"type": "a.done"}],
        "functions": {"f": {"driver": "x"}},
        "config": {"k": "1"},
    }))
    .unwrap();
    let b = Campaign::from_dict(&json!({
        "events": [{"type": "a.done"}, {"type": "b.done"}],
        "functions": {"f": {"driver": "y"}, "g": {"driver": "z"}},
        "config": {"k": "2", "j": "3"},
    }))
    .unwrap();
    a.merge(&b);
    assert_eq!(a.events.len(), 2);
    assert_eq!(a.functions.len(), 2);
    assert_eq!(a.functions[0].driver, "x");
    assert_eq!(a.config["k"], "1");
    assert_eq!(a.config["j"], "3");
}

#[test]
fn campaign_to_dict_round_trips_canonical_shape() {
    let campaign = Campaign::from_dict(&json!({
        "name": "w",
        "nodes": [{"id": "n", "driver": "shell", "on": ["start"]}],
        "stop_on": ["done"],
        "strict": true,
    }))
    .unwrap();
    let dict = campaign.to_dict();
    assert_eq!(dict["name"], "w");
    assert_eq!(dict["nodes"][0]["id"], "n");
    assert_eq!(dict["nodes"][0]["mode"], "any");
    assert_eq!(dict["nodes"][0]["params"], json!({}));
    assert!(dict["nodes"][0].get("max_fires").is_none());
    assert_eq!(dict["stop_on"], json!(["done"]));
    assert_eq!(dict["strict"], true);
    let reparsed = Campaign::from_dict(&dict).unwrap();
    assert_eq!(reparsed.nodes[0].id, "n");
}

// ---------------------------------------------------------------------------
// driver: schedule normalization (ADR-0018)
// ---------------------------------------------------------------------------

#[test]
fn schedule_node_gets_fire_retrigger_and_unlimited_fires() {
    let node = Node::from_dict(&json!({
        "id": "daily",
        "driver": "schedule",
        "on": ["start"],
        "emits": ["report.due"],
        "params": {"cron": "0 9 * * *"},
    }));
    // The synthetic re-arm trigger joins `on`, and both internal
    // kinds join `emits` (the strict-mode contract — `{id}.scheduled`
    // is a returned event).
    assert_eq!(node.on, vec!["start", "daily.fire"]);
    assert_eq!(
        node.emits,
        vec!["report.due", "daily.fire", "daily.scheduled"]
    );
    assert_eq!(node.max_fires, MaxFires::Unlimited);
}

#[test]
fn schedule_node_keeps_explicit_max_fires_and_declared_internals() {
    let node = Node::from_dict(&json!({
        "id": "daily",
        "driver": "schedule",
        "on": ["start", "daily.fire"],
        "emits": ["report.due", "daily.fire", "daily.scheduled"],
        "max_fires": 3,
    }));
    assert_eq!(node.on, vec!["start", "daily.fire"]);
    assert_eq!(node.max_fires, MaxFires::Fires(3));
}

#[test]
fn schedule_node_without_emits_still_self_retriggers() {
    let node = Node::from_dict(&json!({
        "id": "daily",
        "driver": "schedule",
        "on": ["start"],
    }));
    assert!(node.on.contains(&"daily.fire".to_string()));
}

#[test]
fn campaign_runs_on_parses_and_roundtrips() {
    let campaign = Campaign::from_dict(&json!({
        "name": "fleet",
        "runs_on": ["runner:linux", "gpu"],
    }))
    .unwrap();
    assert_eq!(campaign.runs_on, ["runner:linux", "gpu"]);
    let dict = campaign.to_dict();
    assert_eq!(dict["runs_on"], json!(["runner:linux", "gpu"]));
    // Absent runs_on stays absent in the dict.
    let plain = Campaign::from_dict(&json!({"name": "any"})).unwrap();
    assert!(plain.runs_on.is_empty());
    assert!(plain.to_dict().get("runs_on").is_none());
}

#[test]
fn runs_on_matches_requires_every_label() {
    let campaign = Campaign::from_dict(&json!({"runs_on": ["runner:linux", "gpu"]})).unwrap();
    let labels = |v: &[&str]| v.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    assert!(campaign.runs_on_matches(&labels(&["runner:linux", "gpu", "ephemeral"])));
    assert!(!campaign.runs_on_matches(&labels(&["runner:linux"])));
    assert!(!campaign.runs_on_matches(&labels(&[])));
    // Empty runs_on matches anywhere.
    let open = Campaign::from_dict(&json!({"name": "open"})).unwrap();
    assert!(open.runs_on_matches(&labels(&[])));
}
