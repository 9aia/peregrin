+++
id = "I-022"
title = "params.schema.json validation not wired — README overstates"
status = "done"
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

**Done** (merged `bd79a462`, 2026-11-20). `params.schema.json`
(draft 2020-12 via `jsonschema`) is enforced at both boundaries:
`peregrin-loader-yaml` rejects violating `config:` at load when a
schema sits beside the file, and `peregrin provision
install`/`deploy` validates the effective config — bundle defaults
deep-merged under `TAMIAS_DEPLOY_OVERRIDES` (union semantics keep
`required:` honest). Same commit repaired main: release bump had
left workspace dep reqs at 0.2.0 vs package 0.3.0.
