//! peregrin-driver-schedule — `driver: delay` and `driver: schedule`,
//! the temporal drivers (ADR-0018; the ADR's `handl-driver-schedule`
//! package lands here under the `peregrin-*` naming).
//!
//! Neither driver sleeps: `invoke` computes a fire time and queues
//! the emission through [`ExecutionContext::schedule_at`], returning
//! immediately — the runner's timer heap publishes the event when
//! the deadline lands and holds the run open until it does (the
//! "internal scheduler" the ADR calls for). Deferred emissions from
//! a node retired via `retire_on` are dropped; `workflow.cancel`
//! abandons them like the TS `Effect.sleep` race.
//!
//! ## `driver: delay`
//!
//! ```yaml
//! - id: cooldown
//!   driver: delay
//!   on: [deploy.failed]
//!   emits: [cooldown.done]
//!   params:
//!     duration: 5m
//! ```
//!
//! `invoke` arms `emits[0]` for `now + duration` and returns a
//! `{id}.delayed` readiness event (strict campaigns declare it in
//! `emits` alongside the real one). The deferred emission carries
//! `{duration, duration_ms, until}` — `until` an RFC 3339 timestamp.
//! `duration` is a `<n><unit>` string (`500ms`, `30s`, `5m`, `1h`,
//! `2d`; segments compose — `1h30m`); a bare number reads as
//! seconds. Node-level `max_fires`/`until`/`retire_on` apply through
//! the usual trigger machinery: each trigger arms one delay, so
//! `max_fires: N` caps the armed delays.
//!
//! `params.passthrough: true` turns the deferred emission into a
//! `forward:`-style relay: `emits[0]` carries the trigger's payload
//! *verbatim* instead of the delay meta — one deferred emission per
//! trigger, the same per-trigger re-emission `forward:` performs at
//! invoke time (a `mode: all` fan-in relays every collected
//! trigger). The meta isn't lost — it stays on the immediate
//! `{id}.delayed` readiness event. This is the retry-backoff shape:
//! the deferred event IS the re-emitted request, so no `forward:`
//! wiring is needed (a `forward:` would re-emit at invoke time and
//! defeat the wait).
//!
//! ## `driver: schedule`
//!
//! ```yaml
//! - id: daily-report
//!   driver: schedule
//!   on: [start]
//!   emits: [report.due]
//!   params:
//!     cron: "0 9 * * *"
//!     timezone: America/New_York   # optional, default UTC
//! ```
//!
//! `invoke` arms `emits[0]` for the next cron occurrence and returns
//! `{id}.scheduled {cron, timezone, next_fire}`. Recurrence is
//! expressed through the event graph itself: each fire travels as a
//! pair — `emits[0]` for downstream listeners plus a synthetic
//! `{id}.fire` the loader added to `on` (the `poll:` `<id>.tick`
//! convention), whose landing re-invokes the driver to arm the next
//! occurrence. `max_fires` bounds emissions exactly (`1` = one-shot,
//! `unlimited` — a `schedule` node's default — = recurring), and
//! `until`/`retire_on` stop the loop through the usual machinery.
//! Nodes built by hand rather than `Node::from_dict` need
//! `{id}.fire` in `on` themselves to recur. `cron` is standard
//! 5-field cron (`min hour dom mon dow`) with an optional leading
//! seconds field; `timezone` is an IANA name resolved through
//! `chrono-tz`. There is no catch-up: a missed occurrence just moves
//! the next fire forward.
//!
//! Under `--dry-run` both drivers are still invoked
//! ([`Driver::invoke_in_dry_run`]) and report the computed fire time
//! on `emits[0]` (`{dry_run: true, until}` / `{dry_run: true,
//! next_fire}`) instead of arming anything — the ADR's "print the
//! scheduled time rather than actually waiting".

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, SecondsFormat, Utc};
use chrono_tz::Tz;
use croner::Cron;
use croner::parser::{CronParser, Seconds};
use serde_json::{Map, Value};

use peregrin_core::{Event, EventSchema, Node};
use peregrin_driver::{Driver, DriverError, ExecutionContext};
use peregrin_template::{
    Context, Effects, EventScope, NodeScope, Options, ParamsScope, Scope, SystemEffects,
    resolve_value_opts,
};

/// `emits[0]` — the `driver`-convention fallback (`{id}.done`).
fn emit_kind(node: &Node) -> String {
    node.emits
        .first()
        .cloned()
        .unwrap_or_else(|| format!("{}.done", node.id))
}

/// RFC 3339 with milliseconds — the `until`/`next_fire` wire shape.
fn rfc3339(at: &DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// `params.duration` → [`Duration`] — `<n><unit>` segments summed
/// (`90s`, `1h30m`, `250ms`; units `ms`/`s`/`m`/`h`/`d`), or a bare
/// number read as seconds. Returns the reason on failure.
pub fn parse_duration(raw: &str) -> Result<Duration, String> {
    let text = raw.trim();
    if text.is_empty() {
        return Err("empty duration".to_string());
    }
    if let Ok(secs) = text.parse::<f64>() {
        if secs.is_finite() && secs >= 0.0 {
            return Ok(Duration::from_secs_f64(secs));
        }
        return Err(format!("invalid duration '{raw}'"));
    }
    let mut total = 0.0_f64;
    let mut chars = text.chars().peekable();
    let mut any = false;
    while chars.peek().is_some() {
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        let mut num = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() || c == '.' {
                num.push(c);
                chars.next();
            } else {
                break;
            }
        }
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        let mut unit = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_alphabetic() {
                unit.push(c);
                chars.next();
            } else {
                break;
            }
        }
        if num.is_empty() {
            return Err(format!("invalid duration '{raw}'"));
        }
        let factor = match unit.as_str() {
            "ms" => 0.001,
            "s" => 1.0,
            "m" => 60.0,
            "h" => 3600.0,
            "d" => 86400.0,
            _ => {
                return Err(format!(
                    "invalid duration unit '{unit}' in '{raw}' (ms/s/m/h/d)"
                ));
            }
        };
        let value: f64 = num
            .parse()
            .map_err(|_| format!("invalid duration number '{num}' in '{raw}'"))?;
        total += value * factor;
        any = true;
    }
    if !any || !total.is_finite() || total < 0.0 {
        return Err(format!("invalid duration '{raw}'"));
    }
    Ok(Duration::from_secs_f64(total))
}

/// `params.duration` as a number too — numeric params read as
/// seconds (the `input` contract declares `string`, so this leniency
/// only matters for direct `invoke` calls).
fn duration_param(value: &Value) -> Result<Duration, String> {
    match value {
        Value::String(text) => parse_duration(text),
        Value::Number(n) => n
            .as_f64()
            .filter(|s| s.is_finite() && *s >= 0.0)
            .map(Duration::from_secs_f64)
            .ok_or_else(|| format!("invalid duration '{value}'")),
        _ => Err(format!("invalid duration '{value}'")),
    }
}

/// `params.cron` → a [`Cron`] — standard 5-field cron with an
/// optional leading seconds field (6-field), so `* * * * * *` fires
/// every second.
pub fn parse_cron(expr: &str) -> Result<Cron, String> {
    CronParser::builder()
        .seconds(Seconds::Optional)
        .build()
        .parse(expr)
        .map_err(|error| error.to_string())
}

/// The next occurrence of `cron` after `from` — `None` when the
/// pattern can't match within croner's search horizon (a
/// `Feb 30`-style expression). Missed occurrences are skipped, never
/// caught up.
#[must_use]
pub fn next_fire(cron: &Cron, from: &DateTime<Tz>) -> Option<DateTime<Tz>> {
    cron.find_next_occurrence(from, false).ok()
}

/// Re-resolve `node.params` inside `invoke` the way every impl does
/// — `event.*` binds the first trigger so `cron`/`duration` can
/// template off whatever woke the node. Shared by both drivers.
fn resolve_params(
    effects: &Arc<dyn Effects>,
    node: &Node,
    triggers: &[Event],
    ctx: &ExecutionContext,
) -> Result<Map<String, Value>, DriverError> {
    let tctx = Context {
        strict: ctx.strict,
        missing_secrets: ctx.missing_secrets.clone(),
        capabilities: (*ctx.capabilities).clone(),
        effects: &**effects,
    };
    let state_view = |key: &str| ctx.state.borrow().get(key);
    let secret_view = |name: &str| ctx.resolve_secret(name);
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
                secrets: Some(&secret_view),
            },
        },
    )
    .map_err(|error| DriverError::new(error.to_string()))?;
    Ok(resolved.as_object().cloned().unwrap_or_default())
}

fn schema(fields: &[(&str, &str)]) -> EventSchema {
    EventSchema {
        kind: String::new(),
        fields: fields
            .iter()
            .map(|(k, t)| ((*k).to_string(), Value::String((*t).to_string())))
            .collect(),
        schema: Map::new(),
        annotations: Map::new(),
    }
}

/// `driver: delay` — arms `emits[0]` for `now + params.duration`
/// without blocking the run thread.
pub struct DelayDriver {
    effects: Arc<dyn Effects>,
}

impl Default for DelayDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl DelayDriver {
    /// A driver over [`SystemEffects`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            effects: Arc::new(SystemEffects),
        }
    }

    /// A driver over custom effects (tests).
    #[must_use]
    pub fn with_effects(effects: Arc<dyn Effects>) -> Self {
        Self { effects }
    }
}

impl Driver for DelayDriver {
    fn name(&self) -> &'static str {
        "delay"
    }

    fn input(&self) -> Option<EventSchema> {
        Some(schema(&[
            ("duration", "string"),
            ("passthrough", "boolean?"),
        ]))
    }

    fn invoke_in_dry_run(&self) -> bool {
        true
    }

    fn invoke(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let params = resolve_params(&self.effects, node, triggers, ctx)?;
        let Some(raw) = params.get("duration") else {
            return Err(DriverError::new(format!(
                "delay node '{}' missing 'params.duration'",
                node.id
            )));
        };
        let duration = duration_param(raw)
            .map_err(|e| DriverError::new(format!("delay node '{}': {e}", node.id)))?;
        let passthrough = params
            .get("passthrough")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let until = Utc::now() + chrono::Duration::from_std(duration).unwrap_or_default();
        let until = rfc3339(&until);
        let emit = emit_kind(node);
        let meta = serde_json::json!({
            "duration": raw.clone(),
            "duration_ms": u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
            "until": until,
        });
        if ctx.dry_run {
            let mut payload = meta.clone();
            payload["dry_run"] = Value::Bool(true);
            if passthrough {
                payload["passthrough"] = Value::Bool(true);
            }
            return Ok(vec![Event::new(emit, payload, &node.id)]);
        }
        if passthrough && !triggers.is_empty() {
            // `forward:`-style relay: one deferred `emits[0]` per
            // trigger, each carrying that trigger's payload verbatim
            // — the delay meta stays on the `{id}.delayed` readiness
            // event below.
            for trigger in triggers {
                ctx.schedule_in(
                    Event::new(emit.clone(), trigger.payload.clone(), &node.id),
                    duration,
                );
            }
        } else {
            ctx.schedule_in(Event::new(emit, meta, &node.id), duration);
        }
        Ok(vec![Event::new(
            format!("{}.delayed", node.id),
            serde_json::json!({
                "duration": raw.clone(),
                "duration_ms": u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
                "until": until,
            }),
            &node.id,
        )])
    }
}

/// `driver: schedule` — emits `emits[0]` at each `params.cron`
/// occurrence in `params.timezone`; recurrence rides the event
/// graph (see the crate docs).
pub struct ScheduleDriver {
    effects: Arc<dyn Effects>,
}

impl Default for ScheduleDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl ScheduleDriver {
    /// A driver over [`SystemEffects`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            effects: Arc::new(SystemEffects),
        }
    }

    /// A driver over custom effects (tests).
    #[must_use]
    pub fn with_effects(effects: Arc<dyn Effects>) -> Self {
        Self { effects }
    }

    /// `(expr, tz_name, cron, tz)` resolved and validated — shared
    /// by the live and dry-run paths.
    fn spec(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<(String, String, Cron, Tz), DriverError> {
        let params = resolve_params(&self.effects, node, triggers, ctx)?;
        let err = |reason: &str| DriverError::new(format!("schedule node '{}': {reason}", node.id));
        let Some(expr) = params
            .get("cron")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        else {
            return Err(err(
                "missing 'params.cron' — a 5- or 6-field cron expression",
            ));
        };
        let cron =
            parse_cron(expr).map_err(|e| err(&format!("invalid 'params.cron' '{expr}': {e}")))?;
        let tz_name = params
            .get("timezone")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .unwrap_or("UTC")
            .to_string();
        let tz = Tz::from_str(&tz_name)
            .map_err(|_| err(&format!("invalid 'params.timezone' '{tz_name}'")))?;
        Ok((expr.to_string(), tz_name, cron, tz))
    }
}

impl Driver for ScheduleDriver {
    fn name(&self) -> &'static str {
        "schedule"
    }

    fn input(&self) -> Option<EventSchema> {
        Some(schema(&[("cron", "string"), ("timezone", "string?")]))
    }

    fn invoke_in_dry_run(&self) -> bool {
        true
    }

    fn invoke(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let (expr, tz_name, cron, tz) = self.spec(node, triggers, ctx)?;
        let next = next_fire(&cron, &Utc::now().with_timezone(&tz));
        let next_iso = next.map(|dt| rfc3339(&dt.with_timezone(&Utc)));
        let emit = emit_kind(node);
        if ctx.dry_run {
            return Ok(vec![Event::new(
                emit,
                serde_json::json!({
                    "dry_run": true,
                    "cron": expr,
                    "timezone": tz_name,
                    "next_fire": next_iso,
                }),
                &node.id,
            )]);
        }
        if let Some(dt) = next {
            let wait = (dt.with_timezone(&Utc) - Utc::now())
                .to_std()
                .unwrap_or(Duration::ZERO);
            // The fire travels as a pair: `emit` reaches downstream
            // listeners, `{id}.fire` re-triggers this node (the
            // loader appended it to `on`) to arm the next occurrence.
            // Armed first → lower seq → `emit` publishes before the
            // re-arm invoke runs.
            ctx.schedule_in(
                Event::new(
                    emit,
                    serde_json::json!({
                        "cron": expr,
                        "timezone": tz_name,
                        "scheduled_for": next_iso,
                    }),
                    &node.id,
                ),
                wait,
            );
            ctx.schedule_in(
                Event::new(
                    format!("{}.fire", node.id),
                    serde_json::json!({ "schedule": true }),
                    &node.id,
                ),
                wait,
            );
        }
        Ok(vec![Event::new(
            format!("{}.scheduled", node.id),
            serde_json::json!({
                "cron": expr,
                "timezone": tz_name,
                "next_fire": next_iso,
            }),
            &node.id,
        )])
    }
}
