+++
id = "I-019"
title = "supersede: parsed nowhere — queue policy built but inert"
status = "done"
created = "2026-11-07"
priority = "high"
+++

# supersede: parsed nowhere — queue policy built but inert

Status: **open** · Created: 2026-11-07

## Description

`peregrin-queue` implements the supersede policy
(`Supersede{key,mode,cancel_running}`, `src/policy.rs`), and
`RunSupervisor` composes queue+ingress — but:

- `supersede:` is **not parsed** — absent from `Campaign::from_dict`
  (`peregrin-core/src/model.rs` ~L1020-1058); `policy.rs` notes
  "wired into the campaign schema in a later phase".
- `RunSupervisor::new` has **no production call site** —
  `peregrin campaign run` bypasses the queue entirely.

Consequence: the `supersede:` block in `libraries/ci-gate/main.yaml`
and every importer-emitted block (`peregrin-cli/src/import.rs`) is
dead config — bupha-parity queue semantics are unexercised
end-to-end.

## Checklist

- [ ] `supersede:` field parsed in `Campaign::from_dict` + schema doc
- [ ] `RunSupervisor` driven by a production path (node event loop
      or `campaign run`)
- [ ] e2e: duplicate trigger → latest-wins/fifo + cancel_running
- [ ] ci-gate `supersede:` block exercised in an integration test

## Resolution

supersede: parsed in Campaign::from_dict (core Supersede/SupersedeMode, strict); campaign run gains supervisor mode when supersede:+ingress nodes — persistent listener run feeds RunSupervisor, worker runs spawned per trigger with pre-injected event, cancel_running cancels active via injector, runs share --history JSONL. Main ce999f71.
