+++
id = "I-021"
title = "sink-events / sink-sturn egress drivers: ship or drop"
status = "open"
created = "2026-11-07"
priority = "medium"
+++

# sink-events / sink-sturn egress drivers: ship or drop

Status: **open** · Created: 2026-11-07

## Description

P3 leftover. `peregrin-sink-github-status` landed (driver under unified registry); verdicts→event/WAL-plane egress does not exist — no driver writes `refs/sturn/*` (only ingress reads them). Decide: ship sink-events/sink-sturn or drop per I-011 precedent.

## Checklist

- [ ] See Description

## Resolution
