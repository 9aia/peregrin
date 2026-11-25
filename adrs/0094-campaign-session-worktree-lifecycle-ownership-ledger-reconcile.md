+++
id = "ADR-0094"
title = "Campaign session/worktree lifecycle: per-run ownership, cleanup ledger, in-service reconcile"
status = "done"
created = "2026-11-23"
owner = "devin"
priority = "high"
+++

## Context

Automatic workflows (`pr-review`, `kill-mutants`) spawn agent
sessions whose `working_directory` is a disposable worktree under a
per-campaign `worktree_base`. Sessions accumulated because cleanup
was best-effort and invisible:

- ~150 stale sessions (105 pr-review, 46 kill-mutants) bloated the
  agent store to ~5.5G; ~50 orphaned worktrees on disk.
- The first cleanup attempt shelled out to a devin-specific sweeper
  (`devin-gc --cwd`) — coupled to one agent store, matched sessions
  by cwd heuristic, ran only if the cleanup node executed, and hid
  failures behind `|| true`. A cancelled/crashed run leaked the
  session forever.
- Session + worktree + lock cannot be deleted in one transaction —
  there was no durable record of *which* session a run owned, so
  nothing could verify, retry, or audit cleanup.

The org boundary already assigns the pieces: sepia owns session
mechanism (`sessions.*` ops over the session plane), lasius owns
worktrees, peregrin owns orchestration policy.

## Decision

Campaigns own the lifecycle of the session they create; peregrin
owns the recovery policy. Three durable artifacts per campaign
`worktree_base`:

1. **Session registry** — `<base>/sessions/<key>` holds the exact
   session id, written by the node that receives `sessions.run`'s
   result. Cleanup deletes by id via `sepia delete
   <id> --from <agent>` — no cwd heuristic.
2. **Cleanup ledger** — `<base>/cleanup-ledger.jsonl`, one JSON
   line per terminal outcome: `{ts, campaign, key, session_id,
   worktree, outcome, retain_until, detail}`. Outcomes:
   `deleted`/`deleted-cwd` (success path), `retained` (failure path,
   `retain_until = now + 7d`), `failed` (delete errored — retried by
   reconcile). This is the atomicity answer: intent→result is
   recorded per resource so incomplete cleanups are retryable.
3. **Reconcile pass** (`peregrin-node::reconcile`, P-054) — runs
   inside `campaign serve`: once at boot, then every
   `PEREGRIN_RECONCILE_INTERVAL_SECS` (default 900; `0` or
   `PEREGRIN_RECONCILE_DISABLE=1` disables). Per pass it spawns a
   fresh `sepia serve` child (a wedged child can't poison later
   passes), `sessions.list`s, and for every session whose cwd sits
   under a hosted campaign's `worktree_base`:
   - latest ledger `retained` past `retain_until` → delete
     ("retention expired");
   - latest ledger `failed` → retry delete ("cleanup retry");
   - no ledger entry but registry record or base-cwd evidence →
     orphan, deleted only when older than
     `PEREGRIN_RECONCILE_GRACE_SECS` (default 1h) — age from
     registry mtime or `updated_at`; no age evidence → keep;
   - `locked`/`busy` sessions and sessions outside every
     `worktree_base` are never touched.
   Deletes go through `SessionPlane::delete_session` with
   `delete_session_store` fallback for agents lacking ACP delete;
   every reconcile delete appends a `deleted` ledger line.

Retention policy: success (`pushed`/`pr.created`/`summarize.clean`)
→ delete immediately; failure/cancel → retain 7 days for debugging
(worktree kept too when dirty — lasius refuses removal);
`push.skipped` (PR merged/moved mid-run) → delete like success.
Cleanup-node session-delete failure exits 4 → `*.cleanup.failed.*`
event → ledger `failed` → reconcile retries — no silent leaks.

## Consequences

- Orphaned sessions from crashes, restarts, and cancel-before-
  cleanup are reaped automatically within ~1h + one interval.
- Failed-run sessions stay inspectable for 7d alongside their
  worktrees; the ledger is the audit trail for both.
- `devin-gc` demotes to a legacy/manual fallback (the no-session-id
  path in cleanup nodes); it is not the primary mechanism and may
  be retired once reconcile proves out.
- Registry/ledger files accumulate under `worktree_base` (small;
  prune policy can come later).
- Reconcile depends on `sepia serve` resolving at pass time; spawn
  failure logs a warning and the next interval retries — it never
  blocks or poisons the campaign service.
- Session metadata tagging (`owner`/`run_id`/`ephemeral` via
  `sessions.patch_meta`) was considered and deferred — the
  registry+ledger pair already provides exact-id ownership; meta
  tags would strengthen orphan attribution if a third workflow
  family appears.
