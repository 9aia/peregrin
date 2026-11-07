//! `driver: council` tests — fake voter drivers registered under
//! `driver:<name>`/`agent`/`human` lookup names exercise the member
//! plumbing; the tally rules, tie-breaker, timeout, and no-quorum
//! paths run off their scripted ballots.
#![allow(clippy::unwrap_used)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use peregrin_core::{Event, Node};
use peregrin_driver::{Driver, DriverError, ExecutionContext};
use peregrin_driver_council::CouncilDriver;
use serde_json::{Map, Value, json};

/// The lookup table the test councils are built over.
type Lookup = HashMap<String, Arc<dyn Driver>>;
/// The shared param-capture sink behind [`RecordingVoter`].
type Seen = Arc<Mutex<Vec<Value>>>;

// ---------------------------------------------------------------------------
// fake voters
// ---------------------------------------------------------------------------

/// Scripted voter — `params.vote` returns `{vote}`, `params.text`
/// returns `{response}` (free-text parsing), `params.fail` errors the
/// invoke, `params.fail_event` returns `node.failed`, `params.sleep_ms`
/// stalls first. `params.capture` (a sink index unused here) is
/// replaced by the [`RecordingVoter`] for param assertions.
struct FakeVoter;

impl Driver for FakeVoter {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn invoke(
        &self,
        node: &Node,
        _triggers: &[Event],
        _ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        scripted_invoke(node, |_params| {})
    }
}

/// A voter that records each synthetic node's params, then answers
/// like [`FakeVoter`].
struct RecordingVoter {
    seen: Seen,
}

impl Driver for RecordingVoter {
    fn name(&self) -> &'static str {
        "recording"
    }

    fn invoke(
        &self,
        node: &Node,
        _triggers: &[Event],
        _ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        scripted_invoke(node, |params| {
            self.seen.lock().unwrap().push(params.clone());
        })
    }
}

fn scripted_invoke(node: &Node, observe: impl FnOnce(&Value)) -> Result<Vec<Event>, DriverError> {
    let params = &node.params;
    observe(params);
    if params.get("fail").and_then(Value::as_bool).unwrap_or(false) {
        return Err(DriverError::new("scripted voter failure"));
    }
    if let Some(ms) = params.get("sleep_ms").and_then(Value::as_f64)
        && ms > 0.0
    {
        std::thread::sleep(Duration::from_secs_f64(ms / 1000.0));
    }
    if params
        .get("fail_event")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(vec![Event::new(
            "node.failed",
            json!({"error": "voter reported failure"}),
            &node.id,
        )]);
    }
    let payload = if let Some(vote) = params.get("vote") {
        json!({"vote": vote})
    } else if let Some(text) = params.get("text") {
        json!({"response": text})
    } else {
        json!({})
    };
    Ok(vec![Event::new("vote.done", payload, &node.id)])
}

// ---------------------------------------------------------------------------
// fixtures
// ---------------------------------------------------------------------------

/// A lookup over one shared fake voter under the names the council
/// resolves — `fake` (`driver:fake`), `agent` (`agent[:<runner>]`),
/// `human`, plus `recording`/`slow` variants.
fn lookup() -> (Lookup, Seen) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let voter: Arc<dyn Driver> = Arc::new(FakeVoter);
    let map = HashMap::from_iter([
        ("fake".to_string(), voter.clone()),
        ("agent".to_string(), voter.clone()),
        ("human".to_string(), voter),
        (
            "recording".to_string(),
            Arc::new(RecordingVoter {
                seen: Arc::clone(&seen),
            }) as Arc<dyn Driver>,
        ),
    ]);
    (map, seen)
}

fn council(params: Value) -> Node {
    Node {
        id: "c".into(),
        driver: "council".into(),
        emits: vec!["c.decided".into(), "c.tied".into(), "c.short".into()],
        params,
        ..Node::default()
    }
}

fn member(driver: &str, params: Value) -> Value {
    Value::Object(Map::from_iter([
        ("driver".to_string(), Value::String(driver.to_string())),
        ("params".to_string(), params),
    ]))
}

fn invoke(node: &Node, map: &HashMap<String, Arc<dyn Driver>>) -> Vec<Event> {
    CouncilDriver::new(map.clone())
        .invoke(node, &[], &ExecutionContext::default())
        .unwrap()
}

fn first(node: &Node, map: &HashMap<String, Arc<dyn Driver>>) -> Event {
    invoke(node, map).into_iter().next().unwrap()
}

// ---------------------------------------------------------------------------
// tally rules
// ---------------------------------------------------------------------------

#[test]
fn unanimous_decision() {
    let (map, _) = lookup();
    let event = first(
        &council(json!({
            "prompt": "ship it?",
            "members": [
                member("fake", json!({"vote": "yes"})),
                member("fake", json!({"vote": "yes"})),
                member("fake", json!({"vote": "yes"})),
            ],
            "threshold": "unanimous",
        })),
        &map,
    );
    assert_eq!(event.kind, "c.decided");
    assert_eq!(event.payload["winner"], "yes");
    assert_eq!(event.payload["quorum"], true);
    assert_eq!(event.payload["counts"], json!({"yes": 3}));
}

#[test]
fn unanimous_split_is_a_tie() {
    let (map, _) = lookup();
    let event = first(
        &council(json!({
            "prompt": "ship it?",
            "members": [
                member("fake", json!({"vote": "yes"})),
                member("fake", json!({"vote": "no"})),
            ],
            "threshold": "unanimous",
        })),
        &map,
    );
    assert_eq!(event.kind, "c.tied");
    assert_eq!(event.payload["winner"], Value::Null);
    assert_eq!(event.payload["quorum"], true);
}

#[test]
fn majority_decision() {
    let (map, _) = lookup();
    let event = first(
        &council(json!({
            "prompt": "ship it?",
            "members": [
                member("fake", json!({"vote": "yes"})),
                member("fake", json!({"vote": "yes"})),
                member("fake", json!({"vote": "no"})),
            ],
        })),
        &map,
    );
    assert_eq!(event.kind, "c.decided");
    assert_eq!(event.payload["winner"], "yes");
    assert_eq!(event.payload["counts"], json!({"no": 1, "yes": 2}));
}

#[test]
fn majority_exact_tie_without_breaker() {
    let (map, _) = lookup();
    let event = first(
        &council(json!({
            "prompt": "ship it?",
            "members": [
                member("fake", json!({"vote": "yes"})),
                member("fake", json!({"vote": "no"})),
            ],
            "threshold": "majority",
        })),
        &map,
    );
    assert_eq!(event.kind, "c.tied");
    assert_eq!(event.payload["winner"], Value::Null);
}

#[test]
fn supermajority_decision_and_shortfall() {
    let (map, _) = lookup();
    // 2/3 ≥ 0.66 → decided.
    let event = first(
        &council(json!({
            "prompt": "p",
            "members": [
                member("fake", json!({"vote": "yes"})),
                member("fake", json!({"vote": "yes"})),
                member("fake", json!({"vote": "no"})),
            ],
            "threshold": "supermajority",
        })),
        &map,
    );
    assert_eq!(event.kind, "c.decided");
    assert_eq!(event.payload["winner"], "yes");
    // 2/4 < 0.66·4 → tie.
    let event = first(
        &council(json!({
            "prompt": "p",
            "members": [
                member("fake", json!({"vote": "yes"})),
                member("fake", json!({"vote": "yes"})),
                member("fake", json!({"vote": "no"})),
                member("fake", json!({"vote": "no"})),
            ],
            "threshold": "supermajority",
        })),
        &map,
    );
    assert_eq!(event.kind, "c.tied");
    assert_eq!(event.payload["winner"], Value::Null);
}

#[test]
fn any_is_plurality() {
    let (map, _) = lookup();
    let event = first(
        &council(json!({
            "prompt": "p",
            "options": ["red", "green", "blue"],
            "members": [
                member("fake", json!({"vote": "red"})),
                member("fake", json!({"vote": "green"})),
                member("fake", json!({"vote": "green"})),
            ],
            "threshold": "any",
        })),
        &map,
    );
    assert_eq!(event.kind, "c.decided");
    assert_eq!(event.payload["winner"], "green");
    // Split lead → tie.
    let event = first(
        &council(json!({
            "prompt": "p",
            "options": ["red", "green", "blue"],
            "members": [
                member("fake", json!({"vote": "red"})),
                member("fake", json!({"vote": "green"})),
            ],
            "threshold": "any",
        })),
        &map,
    );
    assert_eq!(event.kind, "c.tied");
}

#[test]
fn tie_breaker_resolves() {
    let (map, _) = lookup();
    let event = first(
        &council(json!({
            "prompt": "p",
            "members": [
                member("fake", json!({"vote": "yes"})),
                member("fake", json!({"vote": "yes"})),
                member("fake", json!({"vote": "no"})),
                member("fake", json!({"vote": "no"})),
            ],
            "threshold": "majority",
            "tie_breaker": member("fake", json!({"vote": "yes"})),
        })),
        &map,
    );
    assert_eq!(event.kind, "c.decided");
    assert_eq!(event.payload["winner"], "yes");
    assert_eq!(event.payload["counts"], json!({"no": 2, "yes": 3}));
    let votes = event.payload["votes"].as_array().unwrap();
    assert_eq!(votes.len(), 5);
    assert_eq!(votes[4]["tie_breaker"], true);
    assert_eq!(votes[4]["vote"], "yes");
}

#[test]
fn tie_breaker_still_tied_emits_tie() {
    let (map, _) = lookup();
    // unanimous can't be rescued by one extra vote on a split council.
    let event = first(
        &council(json!({
            "prompt": "p",
            "members": [
                member("fake", json!({"vote": "yes"})),
                member("fake", json!({"vote": "no"})),
            ],
            "threshold": "unanimous",
            "tie_breaker": member("fake", json!({"vote": "yes"})),
        })),
        &map,
    );
    assert_eq!(event.kind, "c.tied");
    assert_eq!(event.payload["winner"], Value::Null);
}

// ---------------------------------------------------------------------------
// member specs + prompt/vote plumbing
// ---------------------------------------------------------------------------

#[test]
fn member_specs_map_to_lookup_names() {
    let (map, seen) = lookup();
    let event = first(
        &council(json!({
            "prompt": "p",
            "members": ["agent:devin", "human", "driver:recording"],
        })),
        &map,
    );
    // `agent`/`human` fake voters return no `vote` param → all
    // no-votes → no quorum. The point is the three driver names
    // resolved and ran (one recording captured its params).
    assert_eq!(event.kind, "c.short");
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(
        seen[0]["prompt"],
        "p\n\nReply with exactly one of: yes, no."
    );
    assert_eq!(seen[0]["options"], json!(["yes", "no"]));
}

#[test]
fn agent_member_gets_runner_param() {
    let (map, seen) = lookup();
    let _ = first(
        &council(json!({
            "prompt": "p",
            "members": [{"driver": "agent:claude", "params": {"vote": "yes"}}],
        })),
        &map,
    );
    // `agent` resolves through the shared fake — the `runner` key is
    // set from the spec suffix; re-register a recording voter under
    // `agent` to see it.
    drop(seen);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut map = map;
    map.insert(
        "agent".to_string(),
        Arc::new(RecordingVoter {
            seen: Arc::clone(&seen),
        }),
    );
    let event = first(
        &council(json!({
            "prompt": "p",
            "members": ["agent:claude"],
        })),
        &map,
    );
    assert_eq!(event.kind, "c.short"); // no vote → no quorum
    let seen = seen.lock().unwrap();
    assert_eq!(seen[0]["runner"], "claude");
}

#[test]
fn vote_parsing_prefers_explicit_field_then_text() {
    let (map, _) = lookup();
    // Human-style `response` text, trailing verdict line.
    let event = first(
        &council(json!({
            "prompt": "p",
            "members": [
                member("fake", json!({"text": "Reviewed it all.\nyes"})),
            ],
        })),
        &map,
    );
    assert_eq!(event.kind, "c.decided");
    assert_eq!(event.payload["winner"], "yes");
    // Whole-word match — "no" inside "not" doesn't count.
    let event = first(
        &council(json!({
            "prompt": "p",
            "members": [
                member("fake", json!({"text": "I would not block it — yes."})),
            ],
        })),
        &map,
    );
    assert_eq!(event.payload["winner"], "yes");
    // Unparseable reply → no-vote → no quorum.
    let event = first(
        &council(json!({
            "prompt": "p",
            "members": [
                member("fake", json!({"text": "it depends"})),
            ],
        })),
        &map,
    );
    assert_eq!(event.kind, "c.short");
    assert_eq!(event.payload["votes"][0]["error"], "no parseable vote");
}

#[test]
fn invalid_option_vote_is_a_no_vote() {
    let (map, _) = lookup();
    let event = first(
        &council(json!({
            "prompt": "p",
            "members": [member("fake", json!({"vote": "maybe"}))],
        })),
        &map,
    );
    assert_eq!(event.kind, "c.short");
    assert_eq!(event.payload["votes"][0]["vote"], Value::Null);
}

// ---------------------------------------------------------------------------
// quorum / timeout
// ---------------------------------------------------------------------------

#[test]
fn member_error_is_a_no_vote() {
    let (map, _) = lookup();
    let event = first(
        &council(json!({
            "prompt": "p",
            "members": [
                member("fake", json!({"vote": "yes"})),
                member("fake", json!({"fail": true})),
                member("fake", json!({"fail_event": true})),
            ],
        })),
        &map,
    );
    assert_eq!(event.kind, "c.short");
    assert_eq!(event.payload["quorum"], false);
    assert_eq!(event.payload["winner"], Value::Null);
    let votes = event.payload["votes"].as_array().unwrap();
    assert_eq!(votes.len(), 3);
    assert_eq!(votes[0]["vote"], "yes");
    assert_eq!(votes[1]["vote"], Value::Null);
    assert!(votes[1]["error"].as_str().unwrap().contains("failed"));
    assert_eq!(votes[2]["vote"], Value::Null);
}

#[test]
fn unknown_member_driver_is_a_no_vote() {
    let (map, _) = lookup();
    let event = first(
        &council(json!({
            "prompt": "p",
            "members": ["driver:nope"],
        })),
        &map,
    );
    assert_eq!(event.kind, "c.short");
    assert!(
        event.payload["votes"][0]["error"]
            .as_str()
            .unwrap()
            .contains("no driver registered")
    );
}

#[test]
fn timeout_skips_late_members() {
    let (map, _) = lookup();
    let event = first(
        &council(json!({
            "prompt": "p",
            "timeout": 0.05,
            "members": [
                member("fake", json!({"vote": "yes", "sleep_ms": 200})),
                member("fake", json!({"vote": "yes"})),
            ],
        })),
        &map,
    );
    assert_eq!(event.kind, "c.short");
    assert_eq!(event.payload["quorum"], false);
    let votes = event.payload["votes"].as_array().unwrap();
    assert_eq!(votes.len(), 2);
    assert_eq!(votes[0]["vote"], "yes");
    assert_eq!(votes[1]["error"], "council timeout");
}

#[test]
fn timeout_is_passed_down_to_voters() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let map = HashMap::from_iter([(
        "recording".to_string(),
        Arc::new(RecordingVoter {
            seen: Arc::clone(&seen),
        }) as Arc<dyn Driver>,
    )]);
    let _ = first(
        &council(json!({
            "prompt": "p",
            "timeout": 30,
            "members": ["driver:recording"],
        })),
        &map,
    );
    let seen = seen.lock().unwrap();
    let timeout = seen[0]["timeout"].as_f64().unwrap();
    assert!(timeout > 0.0 && timeout <= 30.0);
    // No council deadline → voters see timeout 0 (no limit).
    let seen = Arc::new(Mutex::new(Vec::new()));
    let map = HashMap::from_iter([(
        "recording".to_string(),
        Arc::new(RecordingVoter {
            seen: Arc::clone(&seen),
        }) as Arc<dyn Driver>,
    )]);
    let _ = first(
        &council(json!({
            "prompt": "p",
            "members": ["driver:recording"],
        })),
        &map,
    );
    let seen = seen.lock().unwrap();
    assert_eq!(seen[0]["timeout"], 0.0);
}

// ---------------------------------------------------------------------------
// emits, payload shape, dry-run, params
// ---------------------------------------------------------------------------

#[test]
fn emit_overrides_and_fallbacks() {
    let (map, _) = lookup();
    let mut node = council(json!({
        "prompt": "p",
        "emit_ok": "approved",
        "members": [member("fake", json!({"vote": "yes"}))],
    }));
    node.emits.clear();
    let event = first(&node, &map);
    assert_eq!(event.kind, "approved");
    // Fallbacks without emits: `{id}.decided` / `{id}.no_quorum`.
    let mut node = council(json!({
        "prompt": "p",
        "members": [member("fake", json!({"vote": "yes"}))],
    }));
    node.emits.clear();
    assert_eq!(first(&node, &map).kind, "c.decided");
    let mut node = council(json!({
        "prompt": "p",
        "members": [member("fake", json!({}))],
    }));
    node.emits.clear();
    assert_eq!(first(&node, &map).kind, "c.no_quorum");
}

#[test]
fn votes_payload_keeps_the_audit_trail() {
    let (map, _) = lookup();
    let event = first(
        &council(json!({
            "prompt": "p",
            "members": [
                member("fake", json!({"vote": "yes"})),
                member("fake", json!({"vote": "no"})),
            ],
        })),
        &map,
    );
    let votes = event.payload["votes"].as_array().unwrap();
    assert_eq!(votes[0]["member"], "fake");
    assert_eq!(votes[0]["event"]["type"], "vote.done");
    assert_eq!(votes[0]["event"]["payload"]["vote"], "yes");
}

#[test]
fn member_label_override() {
    let (map, _) = lookup();
    let event = first(
        &council(json!({
            "prompt": "p",
            "members": [
                {"driver": "fake", "label": "claude", "params": {"vote": "yes"}},
            ],
        })),
        &map,
    );
    assert_eq!(event.payload["votes"][0]["member"], "claude");
}

#[test]
fn dry_run_reports_shape_without_voters() {
    let (map, _) = lookup();
    let ctx = ExecutionContext {
        dry_run: true,
        ..ExecutionContext::default()
    };
    let events = CouncilDriver::new(map.clone())
        .invoke(
            &council(json!({
                "prompt": "p",
                "members": ["agent:claude", "human"],
            })),
            &[],
            &ctx,
        )
        .unwrap();
    assert_eq!(events[0].kind, "c.decided");
    assert_eq!(events[0].payload["dry_run"], true);
    assert_eq!(
        events[0].payload["members"],
        json!(["agent:claude", "human"])
    );
    assert_eq!(events[0].payload["votes"], json!([]));
}

#[test]
fn missing_params_are_driver_errors() {
    let (map, _) = lookup();
    let driver = CouncilDriver::new(map);
    let err = driver
        .invoke(
            &council(json!({"prompt": "p"})),
            &[],
            &ExecutionContext::default(),
        )
        .unwrap_err();
    assert!(err.reason.contains("members"));
    let err = driver
        .invoke(
            &council(json!({"members": ["driver:fake"]})),
            &[],
            &ExecutionContext::default(),
        )
        .unwrap_err();
    assert!(err.reason.contains("prompt"));
    let err = driver
        .invoke(
            &council(json!({
                "prompt": "p",
                "members": ["driver:fake"],
                "threshold": "plurality",
            })),
            &[],
            &ExecutionContext::default(),
        )
        .unwrap_err();
    assert!(err.reason.contains("threshold"));
}

#[test]
fn prompt_file_feeds_the_question() {
    let (map, _) = lookup();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("prompt.txt");
    std::fs::write(&file, "Vote on the proposal").unwrap();
    let event = first(
        &council(json!({
            "prompt_file": file.display().to_string(),
            "members": [{"driver": "recording", "params": {"vote": "yes"}}],
        })),
        &map,
    );
    assert_eq!(event.kind, "c.decided");
}
