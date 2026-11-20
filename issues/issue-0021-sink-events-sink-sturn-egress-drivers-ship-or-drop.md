+++
id = "I-021"
title = "sink-events / sink-sturn egress drivers: ship or drop"
status = "done"
created = "2026-11-07"
priority = "med"
+++

# sink-events / sink-sturn egress drivers: ship or drop

Status: **open** · Created: 2026-11-07

## Description

P3 leftover. `peregrin-sink-github-status` landed (driver under unified registry); verdicts→event/WAL-plane egress does not exist — no driver writes `refs/sturn/*` (only ingress reads them). Decide: ship sink-events/sink-sturn or drop per I-011 precedent.

## Checklist

- [ ] See Description

## Resolution

**DROP both** (2026-11-20, per I-011 precedent).

Verdicts already reach every consumer: `sink-github-status` for the
CI contract, the `kind="campaign"` entity indexDoc for the entity
plane, `/api/events` + `history.jsonl` + `peregrin runs`/`explain`
for feeds and queries. Arbitrary egress is expressible today with
`driver: http`/`shell` subscribed to `.done`/`.failed` —
`libraries/ledger-notify` is the shipped precedent.

`sink-events` is circular: the event plane IS the engine's bus;
verdicts are already subscribable events. `sink-sturn` has no
defined op kind — sturn's `Op` enum is a closed coordination
vocabulary, the only WAL reader (bupha-sturn) scans `Snapshot`
markers only, and no consumer exists for verdicts in the WAL.
Writing `refs/sturn/*` would forge coordination ops and pollute
every agent's stream.

Note: `ci-gate` was retired in `00960a39`; production CI is bupha's
`ci-job` entity plane, not peregrin campaigns — the P3 rationale
is fully served without these drivers.
