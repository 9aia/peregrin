+++
id = "I-012"
title = "glob-vs-exact rulings: emits strict, until/retire_on/stop_on — verify TS semantics via git show d6bf6356~1, then rule"
status = "open"
created = "2026-11-04"
priority = "medium"
+++

# glob-vs-exact rulings: emits strict, until/retire_on/stop_on — verify TS semantics via git show d6bf6356~1, then rule

Status: **open** · Created: 2026-11-04

## Description

What is wrong and steps to reproduce.

## Checklist

- [x] Verify TS semantics via `git show d6bf6356~1` — done 2026-11-04
- [ ] Rule on glob-match `emits` (the only open ruling below)

## Resolution

### Verified answers (2026-11-04)

Checked against the v1 TS source at `d6bf6356~1` and the ported
crates:

- **`emits` strict check — exact in v1.** `node.emits` is compared
  literally against each produced event type; no glob. A glob-match
  `emits` declaration therefore needs a **new ruling** — strict
  campaigns can't declare wildcard/shard emit types today
  (`forward:`ed `shard.*.done` re-emissions can't satisfy an exact
  list). Contract doc: `docs/reference/event-envelope.md`.
- **`until` / `retire_on` / `stop_on` — exact in v1.** All three are
  literal `contains` checks against seen event types; only `on:` (and
  `fan_in`, `forward` capture sources, `output:` schema lookup)
  glob-match. No ruling needed — keep exact.
- **`state_key` — never existed as an envelope field.** State keys
  are produced from `node.state.key` and ride inside payloads
  (`node.skipped` carries `{node, state_key}`). The envelope stays
  `{type, payload, source, id, ts}` — closed.
- **Scope resolver — never existed.** v1 tests assert
  `${config.region}` passes through literally; `ac-template-engine`
  only resolved `${name(args)}` function calls. The I-009 blocker
  **stands** but is net-new design, not a port regression — spec now
  at `docs/reference/scope-resolution.md`.
- **Cycle detection — real.** `onWorkflowEnd` carries a `cycle`
  flag, `workflow.cycle_detected` is a reserved synthetic type, and
  v1 shipped `--cycle-window`/`--cycle-threshold`/`--no-cycle-detection`
  CLI flags (observer pattern docs + safety docs reference it).
- **`max_fires: 0` — falls back to the default in both.** v1's
  `parseMaxFires` required a positive integer, same as the port;
  `0`/negative/non-integer → default (1, or 100 for self-triggers).
  Strict-mode rejection of `max_fires: 0` is a new ruling, tracked
  under the ADR-0082 fix list.
