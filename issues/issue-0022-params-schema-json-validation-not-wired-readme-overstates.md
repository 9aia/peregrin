+++
id = "I-022"
title = "params.schema.json validation not wired — README overstates"
status = "open"
created = "2026-11-07"
priority = "med"
+++

# params.schema.json validation not wired — README overstates

Status: **open** · Created: 2026-11-07

## Description

`libraries/{ci-gate,release}/params.schema.json` exists but nothing validates against it at runtime — only `tests/import.rs:316-321` asserts it parses. `libraries/ci-gate/README.md:52` claims validation that does not run. Wire into `create`/`install`/loader so typo`d keys reject.

## Checklist

- [ ] See Description

## Resolution
