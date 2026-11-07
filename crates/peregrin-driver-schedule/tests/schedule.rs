#![allow(clippy::unwrap_used, clippy::pedantic)]
//! `delay`/`schedule` driver tests (ADR-0018) — param parsing,
//! deferred emissions through `ctx.schedule_at`, cron/timezone
//! validation, dry-run previews, and `max_fires`/`until`/`retire_on`
//! bounds through full runs.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{TimeZone, Utc};
use chrono_tz::Tz;
use serde_json::{Value, json};

use peregrin_core::{Event, Node};
use peregrin_driver::{Driver, DriverError, ExecutionContext, NoopDriver};
use peregrin_driver_schedule::{
    DelayDriver, ScheduleDriver, next_fire, parse_cron, parse_duration,
};
use peregrin_engine::{Registry, RunOptions, Runner};
use peregrin_loader::Loader;
use peregrin_loader_yaml::YamlLoader;

/// A node with `driver`/`on`/`emits`/`params` — driver-level invokes
/// don't need the `from_dict` normalization the loader applies.
fn node(id: &str, driver: &str, emits: &[&str], params: Value) -> Node {
    Node {
        id: id.to_string(),
        driver: driver.to_string(),
        on: vec!["start".to_string()],
        emits: emits.iter().map(|s| (*s).to_string()).collect(),
        params,
        ..Node::default()
    }
}

fn ctx() -> ExecutionContext {
    ExecutionContext {
        dry_run: false,
        ..ExecutionContext::default()
    }
}

/// `driver: emit` — emits `params.kind` verbatim with `params.payload`
/// (default `{}`); test scaffolding.
struct EmitDriver;

impl Driver for EmitDriver {
    fn name(&self) -> &'static str {
        "emit"
    }

    fn invoke(
        &self,
        node: &Node,
        _triggers: &[Event],
        _ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        Ok(vec![Event::new(
            node.params
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("emit.done"),
            node.params
                .get("payload")
                .cloned()
                .unwrap_or_else(|| json!({})),
            &node.id,
        )])
    }
}

fn registry() -> Registry {
    Registry::default()
        .with("delay", Arc::new(DelayDriver::new()))
        .with("schedule", Arc::new(ScheduleDriver::new()))
        .with("emit", Arc::new(EmitDriver))
        .with("noop", Arc::new(NoopDriver))
}

fn run(yaml: &str) -> Vec<Event> {
    run_opts(yaml, &RunOptions::default())
}

fn run_opts(yaml: &str, options: &RunOptions) -> Vec<Event> {
    let campaign = YamlLoader.load_str(yaml, Path::new(".")).unwrap();
    Runner::new(registry()).run(&campaign, options).unwrap()
}

fn kinds(log: &[Event]) -> Vec<&str> {
    log.iter().map(|e| e.kind.as_str()).collect()
}

// ---------------------------------------------------------------------------
// parse_duration
// ---------------------------------------------------------------------------

#[test]
fn parse_duration_units_and_segments() {
    assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
    assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
    assert_eq!(parse_duration("500ms").unwrap(), Duration::from_millis(500));
    assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
    assert_eq!(parse_duration("1d").unwrap(), Duration::from_secs(86400));
    assert_eq!(parse_duration("1h30m").unwrap(), Duration::from_secs(5400));
    assert_eq!(parse_duration("1m 30s").unwrap(), Duration::from_secs(90));
    // Bare numbers read as seconds (string or numeric literal).
    assert_eq!(parse_duration("45").unwrap(), Duration::from_secs(45));
    assert_eq!(parse_duration("1.5s").unwrap(), Duration::from_millis(1500));
}

#[test]
fn parse_duration_rejects_garbage() {
    for bad in ["", "abc", "5x", "m", "-5", "5 m s", "1.2.3s"] {
        assert!(parse_duration(bad).is_err(), "{bad} should fail");
    }
}

// ---------------------------------------------------------------------------
// cron / timezone validation
// ---------------------------------------------------------------------------

#[test]
fn parse_cron_accepts_five_and_six_fields() {
    assert!(parse_cron("0 9 * * *").is_ok());
    assert!(parse_cron("*/5 * * * * *").is_ok());
    assert!(parse_cron("not a cron").is_err());
    assert!(parse_cron("61 * * * *").is_err());
}

#[test]
fn next_fire_computes_in_utc() {
    let cron = parse_cron("0 9 * * *").unwrap();
    let from: chrono::DateTime<Tz> = Utc
        .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
        .unwrap()
        .with_timezone(&Tz::UTC);
    let next = next_fire(&cron, &from).unwrap().with_timezone(&Utc);
    assert_eq!(next, Utc.with_ymd_and_hms(2026, 1, 1, 9, 0, 0).unwrap());
}

#[test]
fn next_fire_honors_timezone() {
    let cron = parse_cron("0 9 * * *").unwrap();
    let ny: Tz = "America/New_York".parse().unwrap();
    // Jan 1 2026 is EST (UTC-5): 09:00 local = 14:00 UTC.
    let from = Utc
        .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
        .unwrap()
        .with_timezone(&ny);
    let next = next_fire(&cron, &from).unwrap().with_timezone(&Utc);
    assert_eq!(next, Utc.with_ymd_and_hms(2026, 1, 1, 14, 0, 0).unwrap());
}

#[test]
fn next_fire_none_when_pattern_can_never_match() {
    // Feb 30 — a structurally valid pattern with no occurrence.
    let cron = parse_cron("0 0 30 2 *").unwrap();
    let from = Utc
        .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
        .unwrap()
        .with_timezone(&Tz::UTC);
    assert!(next_fire(&cron, &from).is_none());
}

// ---------------------------------------------------------------------------
// delay — driver level
// ---------------------------------------------------------------------------

#[test]
fn delay_requires_duration() {
    let n = node("cooldown", "delay", &["cooldown.done"], json!({}));
    let err = DelayDriver::new().invoke(&n, &[], &ctx()).unwrap_err();
    assert!(err.reason.contains("params.duration"), "{err}");
}

#[test]
fn delay_rejects_bad_duration() {
    let n = node(
        "cooldown",
        "delay",
        &["cooldown.done"],
        json!({"duration": "soon"}),
    );
    let err = DelayDriver::new().invoke(&n, &[], &ctx()).unwrap_err();
    assert!(err.reason.contains("cooldown"), "{err}");
}

#[test]
fn delay_schedules_emit_and_reports_readiness() {
    let n = node(
        "cooldown",
        "delay",
        &["cooldown.done"],
        json!({"duration": "5m"}),
    );
    let ctx = ctx();
    let out = DelayDriver::new().invoke(&n, &[], &ctx).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].kind, "cooldown.delayed");
    assert_eq!(out[0].payload["duration"], "5m");
    assert_eq!(out[0].payload["duration_ms"], 300_000);
    assert!(out[0].payload["until"].as_str().unwrap().contains('T'));

    let scheduled = ctx.take_scheduled();
    assert_eq!(scheduled.len(), 1);
    assert_eq!(scheduled[0].event.kind, "cooldown.done");
    assert_eq!(scheduled[0].event.source, "cooldown");
    assert_eq!(scheduled[0].event.payload["duration_ms"], 300_000);
    assert!(scheduled[0].at > Instant::now());
}

#[test]
fn delay_dry_run_reports_until_without_scheduling() {
    let n = node(
        "cooldown",
        "delay",
        &["cooldown.done"],
        json!({"duration": "5m"}),
    );
    let ctx = ExecutionContext {
        dry_run: true,
        ..ExecutionContext::default()
    };
    let out = DelayDriver::new().invoke(&n, &[], &ctx).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].kind, "cooldown.done");
    assert_eq!(out[0].payload["dry_run"], true);
    assert!(out[0].payload["until"].as_str().unwrap().contains('T'));
    assert!(ctx.take_scheduled().is_empty());
}

// ---------------------------------------------------------------------------
// delay — passthrough
// ---------------------------------------------------------------------------

#[test]
fn delay_passthrough_defers_trigger_payload_verbatim() {
    let n = node(
        "backoff",
        "delay",
        &["task.request"],
        json!({"duration": "50ms", "passthrough": true}),
    );
    let ctx = ctx();
    let trigger = Event::new(
        "task.retry-wait",
        json!({"attempt": 1, "error": "boom"}),
        "retry.loop",
    );
    let out = DelayDriver::new().invoke(&n, &[trigger], &ctx).unwrap();
    // The readiness event still carries the delay meta — it isn't
    // lost, just not the deferred payload.
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].kind, "backoff.delayed");
    assert_eq!(out[0].payload["duration_ms"], 50);

    let scheduled = ctx.take_scheduled();
    assert_eq!(scheduled.len(), 1);
    assert_eq!(scheduled[0].event.kind, "task.request");
    assert_eq!(scheduled[0].event.source, "backoff");
    assert_eq!(
        scheduled[0].event.payload,
        json!({"attempt": 1, "error": "boom"}),
        "deferred payload must be the trigger's verbatim"
    );
    assert!(scheduled[0].at > Instant::now());
}

#[test]
fn delay_passthrough_relays_every_trigger_like_forward() {
    // `mode: all` fan-in: `forward:` re-emits per trigger, so
    // passthrough defers one emit per trigger, payloads in order.
    let n = node(
        "backoff",
        "delay",
        &["task.request"],
        json!({"duration": "10ms", "passthrough": true}),
    );
    let ctx = ctx();
    let triggers = vec![
        Event::new("a", json!({"n": 1}), "src"),
        Event::new("b", json!({"n": 2}), "src"),
    ];
    DelayDriver::new().invoke(&n, &triggers, &ctx).unwrap();
    let scheduled = ctx.take_scheduled();
    assert_eq!(scheduled.len(), 2);
    assert_eq!(scheduled[0].event.payload, json!({"n": 1}));
    assert_eq!(scheduled[1].event.payload, json!({"n": 2}));
    assert!(scheduled.iter().all(|s| s.event.kind == "task.request"));
}

#[test]
fn delay_passthrough_without_triggers_falls_back_to_meta() {
    // A bare driver invoke has no payload to relay — the meta
    // payload keeps the arm observable.
    let n = node(
        "backoff",
        "delay",
        &["task.request"],
        json!({"duration": "10ms", "passthrough": true}),
    );
    let ctx = ctx();
    DelayDriver::new().invoke(&n, &[], &ctx).unwrap();
    let scheduled = ctx.take_scheduled();
    assert_eq!(scheduled.len(), 1);
    assert_eq!(scheduled[0].event.payload["duration_ms"], 10);
}

#[test]
fn delay_passthrough_dry_run_marks_the_relay() {
    let n = node(
        "backoff",
        "delay",
        &["task.request"],
        json!({"duration": "5m", "passthrough": true}),
    );
    let ctx = ExecutionContext {
        dry_run: true,
        ..ExecutionContext::default()
    };
    let trigger = Event::new("task.retry-wait", json!({"attempt": 1}), "src");
    let out = DelayDriver::new().invoke(&n, &[trigger], &ctx).unwrap();
    assert_eq!(out[0].kind, "task.request");
    assert_eq!(out[0].payload["dry_run"], true);
    assert_eq!(out[0].payload["passthrough"], true);
    assert!(ctx.take_scheduled().is_empty());
}

// ---------------------------------------------------------------------------
// schedule — driver level
// ---------------------------------------------------------------------------

#[test]
fn schedule_requires_cron() {
    let n = node("daily", "schedule", &["report.due"], json!({}));
    let err = ScheduleDriver::new().invoke(&n, &[], &ctx()).unwrap_err();
    assert!(err.reason.contains("params.cron"), "{err}");
}

#[test]
fn schedule_rejects_bad_cron_and_timezone() {
    let n = node(
        "daily",
        "schedule",
        &["report.due"],
        json!({"cron": "banana"}),
    );
    let err = ScheduleDriver::new().invoke(&n, &[], &ctx()).unwrap_err();
    assert!(err.reason.contains("cron"), "{err}");

    let n = node(
        "daily",
        "schedule",
        &["report.due"],
        json!({"cron": "0 9 * * *", "timezone": "Mars/Olympus"}),
    );
    let err = ScheduleDriver::new().invoke(&n, &[], &ctx()).unwrap_err();
    assert!(err.reason.contains("timezone"), "{err}");
}

#[test]
fn schedule_arms_next_occurrence() {
    let n = node(
        "daily",
        "schedule",
        &["report.due"],
        json!({"cron": "0 9 * * *", "timezone": "America/New_York"}),
    );
    let ctx = ctx();
    let out = ScheduleDriver::new().invoke(&n, &[], &ctx).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].kind, "daily.scheduled");
    assert_eq!(out[0].payload["cron"], "0 9 * * *");
    assert_eq!(out[0].payload["timezone"], "America/New_York");
    assert!(out[0].payload["next_fire"].is_string());

    let scheduled = ctx.take_scheduled();
    // The fire pair: the emit first, then the `{id}.fire` re-arm.
    assert_eq!(scheduled.len(), 2);
    assert_eq!(scheduled[0].event.kind, "report.due");
    assert_eq!(scheduled[0].event.source, "daily");
    assert!(scheduled[0].event.payload["scheduled_for"].is_string());
    assert_eq!(scheduled[1].event.kind, "daily.fire");
    assert!(scheduled[1].at.saturating_duration_since(scheduled[0].at) < Duration::from_secs(1));
    assert!(scheduled[0].at > Instant::now());
}

#[test]
fn schedule_never_firing_cron_arms_nothing() {
    let n = node(
        "never",
        "schedule",
        &["report.due"],
        json!({"cron": "0 0 30 2 *"}),
    );
    let ctx = ctx();
    let out = ScheduleDriver::new().invoke(&n, &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "never.scheduled");
    assert!(out[0].payload["next_fire"].is_null());
    assert!(ctx.take_scheduled().is_empty());
}

#[test]
fn schedule_dry_run_reports_next_fire_without_scheduling() {
    let n = node(
        "daily",
        "schedule",
        &["report.due"],
        json!({"cron": "0 9 * * *"}),
    );
    let ctx = ExecutionContext {
        dry_run: true,
        ..ExecutionContext::default()
    };
    let out = ScheduleDriver::new().invoke(&n, &[], &ctx).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].kind, "report.due");
    assert_eq!(out[0].payload["dry_run"], true);
    assert_eq!(out[0].payload["timezone"], "UTC");
    assert!(out[0].payload["next_fire"].is_string());
    assert!(ctx.take_scheduled().is_empty());
}

#[test]
fn driver_names_and_dry_run_opt_in() {
    assert_eq!(DelayDriver::new().name(), "delay");
    assert_eq!(ScheduleDriver::new().name(), "schedule");
    assert!(DelayDriver::new().invoke_in_dry_run());
    assert!(ScheduleDriver::new().invoke_in_dry_run());
    assert!(!DelayDriver::new().is_ingress());
    assert!(!ScheduleDriver::new().is_ingress());
}

// ---------------------------------------------------------------------------
// e2e — the deferred emission holds the run until it lands
// ---------------------------------------------------------------------------

#[test]
fn delay_emits_after_duration_without_ingress() {
    let started = Instant::now();
    let log = run("
        nodes:
          - id: cooldown
            driver: delay
            on: [start]
            emits: [cooldown.done, cooldown.delayed]
            params: {duration: 50ms}
          - id: tail
            driver: noop
            on: [cooldown.done]
        ");
    let elapsed = started.elapsed();
    let kinds = kinds(&log);
    let delayed = kinds.iter().position(|k| *k == "cooldown.delayed").unwrap();
    let done = kinds.iter().position(|k| *k == "cooldown.done").unwrap();
    assert!(delayed < done, "order: {kinds:?}");
    assert!(
        elapsed >= Duration::from_millis(40),
        "run ended too early ({elapsed:?}): {kinds:?}"
    );
    assert_eq!(kinds.last(), Some(&"workflow.done"));
    let emit = log.iter().find(|e| e.kind == "cooldown.done").unwrap();
    assert_eq!(emit.payload["duration"], "50ms");
    assert_eq!(emit.payload["duration_ms"], 50);
    assert!(emit.payload["until"].as_str().unwrap().contains('T'));
}

#[test]
fn delay_passthrough_reemits_the_trigger_payload_after_the_wait() {
    // The retry-backoff wiring end to end: the deferred emits[0] IS
    // the re-emitted request — the trigger's payload rides through
    // verbatim, and the run keeps moving during the wait.
    let started = Instant::now();
    let log = run("
        nodes:
          - id: producer
            driver: emit
            on: [start]
            params:
              kind: task.retry-wait
              payload: {attempt: 1, error: boom}
          - id: backoff
            driver: delay
            on: [task.retry-wait]
            emits: [task.request]
            params: {duration: 50ms, passthrough: true}
          - id: tail
            driver: noop
            on: [task.request]
        ");
    let kinds = kinds(&log);
    let request = log.iter().find(|e| e.kind == "task.request").unwrap();
    assert_eq!(
        request.payload,
        json!({"attempt": 1, "error": "boom"}),
        "log: {kinds:?}"
    );
    assert_eq!(request.source, "backoff");
    // Deferred, not immediate: readiness lands first, the request
    // arrives at the deadline.
    let delayed = kinds.iter().position(|k| *k == "backoff.delayed").unwrap();
    let request_at = kinds.iter().position(|k| *k == "task.request").unwrap();
    assert!(delayed < request_at, "order: {kinds:?}");
    assert!(
        started.elapsed() >= Duration::from_millis(40),
        "request arrived before the delay elapsed: {kinds:?}"
    );
    assert_eq!(kinds.last(), Some(&"workflow.done"));
}

#[test]
fn delay_dry_run_short_circuits_the_wait() {
    let started = Instant::now();
    let log = run_opts(
        "
        nodes:
          - id: cooldown
            driver: delay
            on: [start]
            emits: [cooldown.done]
            params: {duration: 30s}
        ",
        &RunOptions {
            dry_run: true,
            ..RunOptions::default()
        },
    );
    let emit = log.iter().find(|e| e.kind == "cooldown.done").unwrap();
    assert_eq!(emit.payload["dry_run"], true);
    assert!(emit.payload["until"].is_string());
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "dry-run waited for the delay"
    );
}

#[test]
fn delay_max_fires_bounds_armed_delays() {
    let log = run("
        nodes:
          - id: a
            driver: emit
            on: [start]
            params: {kind: go}
          - id: b
            driver: emit
            on: [start]
            params: {kind: go2}
          - id: cooldown
            driver: delay
            on: [go, go2]
            emits: [cooldown.done]
            max_fires: 1
            params: {duration: 10ms}
        ");
    let emissions = log.iter().filter(|e| e.kind == "cooldown.done").count();
    assert_eq!(emissions, 1, "log: {:?}", kinds(&log));
}

#[test]
fn delay_retire_on_drops_the_armed_emission() {
    let log = run("
        nodes:
          - id: cooldown
            driver: delay
            on: [start]
            emits: [cooldown.done, cooldown.delayed]
            retire_on: [abort]
            params: {duration: 200ms}
          - id: aborter
            driver: emit
            on: [cooldown.delayed]
            params: {kind: abort}
        ");
    assert!(
        !log.iter().any(|e| e.kind == "cooldown.done"),
        "retired node's armed delay still landed: {:?}",
        kinds(&log)
    );
    assert_eq!(kinds(&log).last(), Some(&"workflow.done"));
}

#[test]
fn schedule_fires_on_cron_and_recurrs_through_max_fires() {
    // Every-second cron (6-field); two emissions then exhausted.
    let log = run("
        nodes:
          - id: ticker
            driver: schedule
            on: [start]
            emits: [tick]
            max_fires: 2
            params: {cron: \"* * * * * *\"}
        ");
    let kinds = kinds(&log);
    let emissions = log.iter().filter(|e| e.kind == "tick").count();
    assert_eq!(emissions, 2, "log: {kinds:?}");
    assert!(kinds.contains(&"ticker.scheduled"));
    assert_eq!(kinds.last(), Some(&"workflow.done"));
    let emit = log.iter().find(|e| e.kind == "tick").unwrap();
    assert_eq!(emit.payload["cron"], "* * * * * *");
    assert!(emit.payload["scheduled_for"].is_string());
}

#[test]
fn schedule_one_shot_with_max_fires_1() {
    let log = run("
        nodes:
          - id: once
            driver: schedule
            on: [start]
            emits: [tick]
            max_fires: 1
            params: {cron: \"* * * * * *\"}
        ");
    let emissions = log.iter().filter(|e| e.kind == "tick").count();
    assert_eq!(emissions, 1, "log: {:?}", kinds(&log));
    assert_eq!(kinds(&log).last(), Some(&"workflow.done"));
}

#[test]
fn schedule_dry_run_prints_next_fire() {
    let started = Instant::now();
    let log = run_opts(
        "
        nodes:
          - id: daily
            driver: schedule
            on: [start]
            emits: [report.due]
            params: {cron: \"0 9 * * *\", timezone: America/New_York}
        ",
        &RunOptions {
            dry_run: true,
            ..RunOptions::default()
        },
    );
    let emit = log.iter().find(|e| e.kind == "report.due").unwrap();
    assert_eq!(emit.payload["dry_run"], true);
    assert_eq!(emit.payload["timezone"], "America/New_York");
    assert!(emit.payload["next_fire"].is_string());
    assert!(started.elapsed() < Duration::from_secs(5));
}
