# ledger-notify

Push notifications for org ledger activity — the peregrin side of
T-025 / PL-016 Phase 4 ("notify assignees" without making myco
push-based).

Every 15 minutes the campaign polls `refs/heads/project` in each
repo clone under `$MYCO_REPOS_DIR` (default the 9aia workspace
layout), diffs moved tips locally (`prev..tip`, subjects + `Actor:`
trailers), and POSTs a digest to `$NTFY_URL` (ntfy.sh or
self-hosted). First run per repo seeds the baseline silently — no
notification storm on deploy.

Env (in `~/.config/peregrin/env`, picked up via `env_file`):

- `NTFY_URL` — **required for notifications**; unset → the sweep
  no-ops cleanly (`sweep.done`, "nothing to do")
- `MYCO_REPOS_DIR` — optional override for the clones dir
- `LEDGER_NOTIFY_STATE` — optional state dir override
  (default `~/.local/share/peregrin/ledger-notify`)

Read-only on ledgers: it shells `git rev-parse`/`git log` inside
each `project/` worktree — no myco verbs, no ledger writes.

Install/deploy like any bundle: `tamias install <ref>` then
`tamias deploy ledger-notify`.
