# myco-maintenance

Scheduled ledger hygiene for the org's myco work ledgers — the
peregrin side of `myco reap` (org T-022 / PL-016 Phase 4).

`main.yaml` is a complete campaign: a `schedule` node fires every
6h (`23 */6 * * *`, America/Sao_Paulo), a `shell` node runs
`myco reap --idle-days 3 --dir <repo>/project --json` once per
ledger under `$MYCO_REPOS_DIR` (default
`~/GitHub/9aia/workspace/repos`), counting per-repo failures
without aborting the sweep.

Prereqs on the host:

- `myco` on the service PATH (via `~/.config/peregrin/env`)
- `MYCO_REPOS_DIR` set there if the clones don't live at the
  default path
- each repo's `project` branch pushed/tracked — `reap` mutates the
  ledger, so `ensure_synced` applies like any verb

Install/deploy like any bundle: `tamias install <ref>` then
`tamias deploy myco-maintenance`.
