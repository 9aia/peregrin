//! peregrin-driver-council — `driver: council` (ADR-0004).
//!
//! One node = a voting round. `params.members` lists the voters —
//! `agent:<runner>`, `human`, or `driver:<name>` (object form
//! `{driver: <spec>, params: {...}, label: "..."}` also accepted, for
//! member-specific driver params). Each member resolves through the
//! [`DriverLookup`] the driver was built with — the runner's
//! `Registry` at the composition level, like `git`'s `params.inner` —
//! and is invoked *inside* `invoke` as a sub-effect on a synthetic
//! node carrying `{prompt, options, timeout}`; the member's explicit
//! `params` merge over those. The voter's returned events yield the
//! vote: `payload.vote` first, else the `response`/`content`/
//! `stdout`/`text` reply is scanned for an option (exact
//! case-insensitive match, then first whole-word occurrence). Agents
//! and humans get the prompt wrapped with "Reply with exactly one
//! of: …" so the reply parses.
//!
//! `params.threshold` — `unanimous` (all votes identical),
//! `majority` (top option > half the votes), `supermajority` (top
//! option ≥ `params.supermajority`, default `0.66`, of the votes), or
//! `any` (plurality — the top option, any lead). A tie runs
//! `params.tie_breaker` once and re-tallies; a persisting tie emits
//! `emit_tie` (`emits[1]`/`{id}.tie`), a decision `emit_ok`
//! (`emits[0]`/`{id}.decided`). `params.timeout` seconds bounds the
//! round — checked between members and passed down as each voter's
//! own `timeout` (0/unset = no limit; a synchronous voter can't be
//! interrupted mid-invoke). Any member that errors, returns no
//! parseable vote, or goes un-polled under the deadline is a
//! no-vote; any no-vote breaks quorum (`quorum` = every member
//! voted) and the round emits `emit_no_quorum`
//! (`emits[2]`/`{id}.no_quorum`) instead of tallying. Every result
//! event carries `{votes, counts, winner, quorum}` — `votes` is one
//! `{member, vote, event, error?, tie_breaker?}` entry per ballot for
//! the audit trail.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{Map, Value};

use peregrin_core::{Event, EventSchema, Node};
use peregrin_driver::{Driver, DriverError, DriverLookup, ExecutionContext};
use peregrin_template::{
    Context, Effects, EventScope, NodeScope, Options, ParamsScope, Scope, SystemEffects,
    resolve_value_opts,
};

/// `params.threshold` default.
const DEFAULT_THRESHOLD: &str = "majority";
/// `params.supermajority` default fraction.
const DEFAULT_SUPERMAJORITY: f64 = 0.66;
/// `params.options` default.
const DEFAULT_OPTIONS: [&str; 2] = ["yes", "no"];
/// Payload keys scanned for a voter's reply text when there is no
/// explicit `vote` field — `response` (human), `content` (litellm),
/// `stdout` (agent runner), then generic fallbacks.
const TEXT_FIELDS: [&str; 5] = ["response", "content", "stdout", "text", "answer"];

/// `driver: council`. `lookup` resolves member driver names (the
/// runner's `Registry` at composition); `effects` backs the param
/// re-resolve and `prompt_file` read.
pub struct CouncilDriver {
    lookup: Arc<dyn DriverLookup>,
    effects: Arc<dyn Effects>,
}

impl CouncilDriver {
    /// A driver over [`SystemEffects`].
    #[must_use]
    pub fn new(lookup: impl DriverLookup + 'static) -> Self {
        Self::with_parts(Arc::new(lookup), Arc::new(SystemEffects))
    }

    /// A driver with every seam explicit (tests).
    #[must_use]
    pub fn with_parts(lookup: Arc<dyn DriverLookup>, effects: Arc<dyn Effects>) -> Self {
        Self { lookup, effects }
    }

    /// Re-resolve `node.params` the way the sibling drivers do inside
    /// `invoke` (the runner resolves too — the driver-level pass keeps
    /// direct invocations working). `ctx` lends its strict flag,
    /// capability map, and shared missing-secret ledger.
    fn resolve_params(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Map<String, Value>, DriverError> {
        let tctx = Context {
            strict: ctx.strict,
            missing_secrets: ctx.missing_secrets.clone(),
            capabilities: (*ctx.capabilities).clone(),
            effects: &*self.effects,
        };
        let state_view = |key: &str| ctx.state.borrow().get(key);
        let resolved = resolve_value_opts(
            &node.params,
            &tctx,
            &Options {
                node: Some(node),
                functions: &[],
                scope: Scope {
                    event: triggers.first().map(|event| EventScope {
                        event,
                        patterns: &node.on,
                    }),
                    config: Some(&ctx.config),
                    params: ParamsScope::Resolved(&ctx.params),
                    state: Some(&state_view),
                    node: Some(NodeScope {
                        id: &node.id,
                        fires: ctx.fires,
                    }),
                    secrets: Some(&|name: &str| ctx.resolve_secret(name)),
                },
            },
        )
        .map_err(|error| DriverError::new(error.to_string()))?;
        Ok(resolved.as_object().cloned().unwrap_or_default())
    }

    /// Invoke one voter driver on a synthetic node and fold the result
    /// into a [`Ballot`]. Voters share `ctx` — `emit`/`inject` side
    /// channels flow into the run like `git`'s inner driver.
    fn run_voter(
        &self,
        call: &VoteCall<'_>,
        member: &Member,
        ordinal: &str,
        remaining: Option<Duration>,
    ) -> Ballot {
        let mut ballot = Ballot {
            member: member.label.clone(),
            tie_breaker: member.tie_breaker,
            vote: None,
            event: None,
            error: None,
        };
        let Some(driver) = self.lookup.driver(&member.driver) else {
            ballot.error = Some(format!("no driver registered for '{}'", member.driver));
            return ballot;
        };
        let mut params = Map::from_iter([
            ("prompt".to_string(), Value::String(call.prompt.to_string())),
            (
                "options".to_string(),
                Value::Array(
                    call.options
                        .iter()
                        .map(|option| Value::String(option.clone()))
                        .collect(),
                ),
            ),
            (
                // The round's remaining budget bounds the voter's own
                // `timeout`; no council deadline means `0` (no limit),
                // matching `params.timeout`'s semantics. Members
                // override via their own `params`.
                "timeout".to_string(),
                Value::from(remaining.map_or(0.0, |left| left.as_secs_f64())),
            ),
        ]);
        params.extend(member.params.clone());
        let voter_node = Node {
            id: format!("{}.vote.{ordinal}", call.node.id),
            driver: member.driver.clone(),
            emits: vec![format!("{}.vote", call.node.id)],
            params: Value::Object(params),
            ..Node::default()
        };
        match driver.invoke(&voter_node, call.triggers, call.ctx) {
            Err(error) => {
                ballot.error = Some(format!("voter '{}' failed: {error}", member.driver));
            }
            Ok(events) => {
                ballot.event = events.first().cloned();
                ballot.vote = events
                    .iter()
                    .find_map(|event| extract_vote(event, call.options));
                if ballot.vote.is_none() && ballot.error.is_none() {
                    ballot.error = Some("no parseable vote".to_string());
                }
            }
        }
        ballot
    }
}

/// The pieces every voter invocation in a round shares — bundled so
/// [`CouncilDriver::run_voter`] stays under the arity lint.
struct VoteCall<'a> {
    node: &'a Node,
    triggers: &'a [Event],
    ctx: &'a ExecutionContext,
    prompt: &'a str,
    options: &'a [String],
}

/// A parsed `params.members`/`params.tie_breaker` entry.
struct Member {
    /// Display name in the `votes` payload — the spec string, or the
    /// object's `label`/`driver`.
    label: String,
    /// The [`DriverLookup`] name.
    driver: String,
    /// Member-scoped voter params, merged over the council's
    /// `{prompt, options, timeout}`.
    params: Map<String, Value>,
    /// Set on the `tie_breaker` ballot.
    tie_breaker: bool,
}

/// Parse one member spec — a string (`agent`, `agent:<runner>`,
/// `human`, `driver:<name>` — a bare name also resolves through the
/// lookup) or an object `{driver: <spec>, params?, label?}`.
fn parse_member(raw: &Value) -> Result<Member, String> {
    match raw {
        Value::String(spec) => member_from_spec(spec, Map::new(), None),
        Value::Object(map) => {
            let spec = map
                .get("driver")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if spec.is_empty() {
                return Err("member object missing 'driver'".to_string());
            }
            let params = map
                .get("params")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            member_from_spec(spec, params, map.get("label").and_then(Value::as_str))
        }
        _ => Err("member must be a string or {driver, params?} object".to_string()),
    }
}

/// `agent[:<runner>]` → the `agent` driver (+ `runner` param);
/// `human` → `human`; `driver:<name>` and bare `<name>` → `<name>`.
fn member_from_spec(
    spec: &str,
    mut params: Map<String, Value>,
    label: Option<&str>,
) -> Result<Member, String> {
    let (driver, runner) = if spec == "human" {
        ("human", None)
    } else if spec == "agent" {
        ("agent", None)
    } else if let Some(runner) = spec.strip_prefix("agent:") {
        ("agent", Some(runner))
    } else {
        (spec.strip_prefix("driver:").unwrap_or(spec), None)
    };
    if driver.is_empty() {
        return Err(format!("invalid member spec '{spec}'"));
    }
    if let Some(runner) = runner {
        params
            .entry("runner".to_string())
            .or_insert_with(|| Value::String(runner.to_string()));
    }
    Ok(Member {
        label: label.map_or_else(|| spec.to_string(), str::to_string),
        driver: driver.to_string(),
        params,
        tie_breaker: false,
    })
}

/// The decision rule.
enum Threshold {
    Unanimous,
    Majority,
    Supermajority(f64),
    /// Plurality — the top option wins on any lead.
    Any,
}

fn parse_threshold(raw: Option<&Value>, fraction: f64) -> Result<Threshold, String> {
    let name = raw.and_then(Value::as_str).unwrap_or(DEFAULT_THRESHOLD);
    match name {
        "unanimous" => Ok(Threshold::Unanimous),
        "majority" => Ok(Threshold::Majority),
        "supermajority" => Ok(Threshold::Supermajority(fraction)),
        "any" => Ok(Threshold::Any),
        _ => Err(format!(
            "unknown threshold '{name}' (unanimous|majority|supermajority|any)"
        )),
    }
}

/// `f64` param with a default — absent/non-numeric falls back.
fn number_param(params: &Map<String, Value>, key: &str, default: f64) -> f64 {
    params
        .get(key)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .unwrap_or(default)
}

/// A non-empty string param, else `None`.
fn non_empty_str<'a>(params: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    params
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

/// One voter's outcome — `vote` is the extracted option; `event` is
/// the voter's first returned event for the audit trail; `error`
/// carries the no-vote reason.
struct Ballot {
    member: String,
    tie_breaker: bool,
    vote: Option<String>,
    event: Option<Event>,
    error: Option<String>,
}

fn ballot_json(ballot: &Ballot) -> Value {
    let mut entry = Map::from_iter([
        ("member".to_string(), Value::String(ballot.member.clone())),
        (
            "vote".to_string(),
            ballot
                .vote
                .as_ref()
                .map_or(Value::Null, |vote| Value::String(vote.clone())),
        ),
        (
            "event".to_string(),
            ballot.event.as_ref().map_or(Value::Null, |event| {
                serde_json::to_value(event).unwrap_or_default()
            }),
        ),
    ]);
    if ballot.tie_breaker {
        entry.insert("tie_breaker".to_string(), Value::Bool(true));
    }
    if let Some(error) = &ballot.error {
        entry.insert("error".to_string(), Value::String(error.clone()));
    }
    Value::Object(entry)
}

/// `vote`/`response`/`content`/`stdout`/… → an option, or `None`.
/// Failure-kinded events never carry a vote.
fn extract_vote(event: &Event, options: &[String]) -> Option<String> {
    if event.kind == "node.failed" || event.kind.ends_with(".failed") {
        return None;
    }
    if let Some(raw) = event.payload.get("vote").and_then(Value::as_str) {
        return match_option(raw, options);
    }
    for key in TEXT_FIELDS {
        if let Some(text) = event.payload.get(key).and_then(Value::as_str)
            && let Some(vote) = parse_vote_text(text, options)
        {
            return Some(vote);
        }
    }
    None
}

/// Exact (trimmed, case-insensitive) option match.
fn match_option(text: &str, options: &[String]) -> Option<String> {
    let trimmed = text.trim();
    options
        .iter()
        .find(|option| option.eq_ignore_ascii_case(trimmed))
        .cloned()
}

/// Parse free reply text for an option: first an exact per-line
/// match, then the earliest whole-word occurrence across the text.
/// Substring hits inside other words ("no" in "not") don't count.
fn parse_vote_text(text: &str, options: &[String]) -> Option<String> {
    for line in text.lines() {
        if let Some(vote) = match_option(line, options) {
            return Some(vote);
        }
    }
    let lower = text.to_lowercase();
    let mut best: Option<(usize, String)> = None;
    for option in options {
        let needle = option.to_lowercase();
        for (idx, _) in lower.match_indices(&needle) {
            let before = lower[..idx].chars().next_back();
            let after = lower[idx + needle.len()..].chars().next();
            let bounded = before.is_none_or(|c| !c.is_alphanumeric())
                && after.is_none_or(|c| !c.is_alphanumeric());
            if bounded {
                if best.as_ref().is_none_or(|(at, _)| idx < *at) {
                    best = Some((idx, option.clone()));
                }
                break;
            }
        }
    }
    best.map(|(_, option)| option)
}

/// `option → count` over the ballots that voted.
fn tally(ballots: &[Ballot]) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for vote in ballots.iter().filter_map(|ballot| ballot.vote.as_ref()) {
        *counts.entry(vote.clone()).or_insert(0) += 1;
    }
    counts
}

/// Apply the threshold to the tally — `Some(winner)` on a decision.
fn decide(threshold: &Threshold, counts: &BTreeMap<String, u64>) -> Option<String> {
    let total: u64 = counts.values().sum();
    let top = counts.values().max().copied().unwrap_or(0);
    let leaders: Vec<&String> = counts
        .iter()
        .filter(|(_, count)| **count == top)
        .map(|(option, _)| option)
        .collect();
    let unique = (leaders.len() == 1).then(|| leaders[0].clone());
    match threshold {
        Threshold::Unanimous => (top == total && leaders.len() == 1).then(|| leaders[0].clone()),
        Threshold::Majority => unique.filter(|_| top * 2 > total),
        Threshold::Supermajority(fraction) => {
            unique.filter(|_| (top as f64) >= fraction * (total as f64))
        }
        Threshold::Any => unique,
    }
}

/// The `{votes, counts, winner, quorum}` result payload — `winner`
/// is the decided option, `null` on a tie or a missed quorum.
fn result_payload(ballots: &[Ballot], quorum: bool, winner: Option<&str>) -> Map<String, Value> {
    let counts = tally(ballots);
    Map::from_iter([
        (
            "votes".to_string(),
            Value::Array(ballots.iter().map(ballot_json).collect()),
        ),
        (
            "counts".to_string(),
            Value::Object(
                counts
                    .iter()
                    .map(|(option, count)| {
                        (
                            option.clone(),
                            Value::Number(serde_json::Number::from(*count)),
                        )
                    })
                    .collect(),
            ),
        ),
        (
            "winner".to_string(),
            winner.map_or(Value::Null, |option| Value::String(option.to_string())),
        ),
        ("quorum".to_string(), Value::Bool(quorum)),
    ])
}

/// `input` contract — `members` required; `prompt`/`prompt_file` are
/// either-or, so both stay optional here (the driver errors when both
/// are absent). Member/tie-breaker specs accept several shapes and
/// stay open (`{}`).
fn input_schema() -> EventSchema {
    let schema = serde_json::json!({
        "type": "object",
        "required": ["members"],
        "properties": {
            "members": {"type": "array"},
            "prompt": {"type": "string"},
            "prompt_file": {"type": "string"},
            "options": {"type": "array"},
            "threshold": {"type": "string"},
            "supermajority": {"type": "number"},
            "timeout": {},
            "tie_breaker": {},
            "emit_ok": {"type": "string"},
            "emit_tie": {"type": "string"},
            "emit_no_quorum": {"type": "string"},
        },
    });
    EventSchema {
        kind: String::new(),
        fields: Map::new(),
        schema: schema.as_object().cloned().unwrap_or_default(),
        annotations: Map::new(),
    }
}

impl Driver for CouncilDriver {
    fn name(&self) -> &'static str {
        "council"
    }

    fn input(&self) -> Option<EventSchema> {
        Some(input_schema())
    }

    /// Opt in so `--dry-run` reports the council's shape — member
    /// list and result skeleton — without calling any voter.
    fn invoke_in_dry_run(&self) -> bool {
        true
    }

    fn invoke(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let params = self.resolve_params(node, triggers, ctx)?;
        let fail = |error: String| DriverError::new(format!("council node '{}': {error}", node.id));

        let members: Vec<Member> = params
            .get("members")
            .and_then(Value::as_array)
            .ok_or_else(|| fail("missing 'params.members' list".to_string()))?
            .iter()
            .map(parse_member)
            .collect::<Result<_, _>>()
            .map_err(fail)?;
        if members.is_empty() {
            return Err(fail("'params.members' must not be empty".to_string()));
        }

        let options: Vec<String> = match params.get("options") {
            None | Some(Value::Null) => DEFAULT_OPTIONS.iter().map(|s| (*s).to_string()).collect(),
            Some(value) => value
                .as_array()
                .ok_or_else(|| fail("'params.options' must be a list".to_string()))?
                .iter()
                .map(|option| {
                    option
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .ok_or_else(|| {
                            fail("'params.options' entries must be non-empty strings".to_string())
                        })
                })
                .collect::<Result<_, _>>()?,
        };
        if options.is_empty() {
            return Err(fail("'params.options' must not be empty".to_string()));
        }

        let mut prompt = non_empty_str(&params, "prompt")
            .map(str::to_string)
            .unwrap_or_default();
        if let Some(prompt_file) = non_empty_str(&params, "prompt_file") {
            let path = if Path::new(prompt_file).is_absolute() {
                PathBuf::from(prompt_file)
            } else {
                self.effects.cwd().join(prompt_file)
            };
            let bytes = self
                .effects
                .read_file(&path)
                .map_err(|error| fail(format!("failed to read prompt_file: {error}")))?;
            let contents = String::from_utf8_lossy(&bytes);
            if !contents.is_empty() {
                prompt = format!("{contents}\n{prompt}");
            }
        }
        if prompt.is_empty() {
            return Err(fail(
                "missing 'params.prompt' or 'params.prompt_file'".to_string(),
            ));
        }
        let voter_prompt = format!(
            "{prompt}\n\nReply with exactly one of: {}.",
            options.join(", ")
        );

        let threshold = parse_threshold(
            params.get("threshold"),
            number_param(&params, "supermajority", DEFAULT_SUPERMAJORITY),
        )
        .map_err(fail)?;

        let emit_ok = non_empty_str(&params, "emit_ok")
            .map(str::to_string)
            .or_else(|| node.emits.first().cloned())
            .unwrap_or_else(|| format!("{}.decided", node.id));
        let emit_tie = non_empty_str(&params, "emit_tie")
            .map(str::to_string)
            .or_else(|| node.emits.get(1).cloned())
            .unwrap_or_else(|| format!("{}.tie", node.id));
        let emit_no_quorum = non_empty_str(&params, "emit_no_quorum")
            .map(str::to_string)
            .or_else(|| node.emits.get(2).cloned())
            .unwrap_or_else(|| format!("{}.no_quorum", node.id));

        if ctx.dry_run {
            let mut payload = result_payload(&[], false, None);
            payload.insert("dry_run".to_string(), Value::Bool(true));
            payload.insert(
                "members".to_string(),
                Value::Array(
                    members
                        .iter()
                        .map(|member| Value::String(member.label.clone()))
                        .collect(),
                ),
            );
            return Ok(vec![Event::new(emit_ok, Value::Object(payload), &node.id)]);
        }

        let timeout_seconds = number_param(&params, "timeout", 0.0);
        let deadline = (timeout_seconds > 0.0)
            .then(|| Instant::now() + Duration::from_secs_f64(timeout_seconds));

        // --- the round: one synchronous voter call per member -------
        let call = VoteCall {
            node,
            triggers,
            ctx,
            prompt: &voter_prompt,
            options: &options,
        };
        let mut ballots: Vec<Ballot> = Vec::new();
        for (index, member) in members.iter().enumerate() {
            let remaining = deadline.map(|at| at.saturating_duration_since(Instant::now()));
            if deadline.is_some() && remaining.is_some_and(|left| left.is_zero()) {
                for member in members.iter().skip(index) {
                    ballots.push(Ballot {
                        member: member.label.clone(),
                        tie_breaker: false,
                        vote: None,
                        event: None,
                        error: Some("council timeout".to_string()),
                    });
                }
                break;
            }
            ballots.push(self.run_voter(&call, member, &index.to_string(), remaining));
        }

        let quorum = ballots.iter().all(|ballot| ballot.vote.is_some());
        if !quorum {
            return Ok(vec![Event::new(
                emit_no_quorum,
                Value::Object(result_payload(&ballots, false, None)),
                &node.id,
            )]);
        }

        // --- decision, with one tie-breaker vote when tied ----------
        let mut winner = decide(&threshold, &tally(&ballots));
        if winner.is_none()
            && let Some(raw) = params.get("tie_breaker")
        {
            let mut breaker = parse_member(raw).map_err(fail)?;
            breaker.tie_breaker = true;
            let remaining = deadline.map(|at| at.saturating_duration_since(Instant::now()));
            if deadline.is_none() || remaining.is_some_and(|left| !left.is_zero()) {
                ballots.push(self.run_voter(&call, &breaker, "breaker", remaining));
                winner = decide(&threshold, &tally(&ballots));
            }
        }

        let kind = if winner.is_some() { emit_ok } else { emit_tie };
        Ok(vec![Event::new(
            kind,
            Value::Object(result_payload(&ballots, true, winner.as_deref())),
            &node.id,
        )])
    }
}
