# Run result — the `campaign run` verdict artifact

`peregrin campaign run` ends every execution by projecting a
**`RunResult`** document — the machine-readable verdict CI gates
read instead of scraping the `ts source type` event log. The
process exit code stays `0` on a *completed* run whatever the
verdict: node failure is data in the artifact, not an exit status.

The document follows the **GitHub Checks model**: `status` is the
lifecycle (`running`/`completed`/`cancelled`) and `conclusion` is
the canonical outcome vocabulary
(`success`/`failure`/`neutral`/`skipped`/`timed_out`/`cancelled`/
`action_required`). `verdict` stays as the campaign's own coarse
label (`pass`/`fail`/`error`) — `error` has no GH-Checks spelling
(the REST enum tops out at `action_required`; `startup_failure` is
GraphQL-only), so campaign-plane failures map to
`"conclusion": "failure"` while `"verdict": "error"` preserves the
distinction.

## Where it lands

- `--result <path>` writes the document to that JSON file.
- `--history <path>` implies `<history>.result.json` — the
  `with_extension("result.json")` sibling, so `runs.jsonl` becomes
  `runs.result.json`. An explicit `--result` wins over the derived
  path.
- Neither flag → no artifact (the pre-flag behavior).
- **Supervisor mode** (`supersede:` + ingress nodes) has no single
  verdict — runs spawn per trigger — so the path is appended to as
  **JSONL**, one `RunResult` object per finished run (a log-only
  projection: `secs`/`fires` are absent).

Campaign-plane failures (loader rejection, engine `Err`) still
exit nonzero *and* leave a `"verdict": "error"` /
`"conclusion": "failure"` document when a result path resolved.

## Shape

```json
{
  "schema": "peregrin/run-result/1",
  "run_id": "9f2c…16hex",
  "campaign": "gate",
  "status": "completed",
  "verdict": "fail",
  "conclusion": "failure",
  "started_at": "2026-02-14T10:00:00.123+00:00",
  "completed_at": "2026-02-14T10:01:24.880+00:00",
  "output": { "title": "gate", "summary": "", "text": "" },
  "nodes": [
    {
      "id": "build",
      "status": "completed",
      "conclusion": "success",
      "fires": 1,
      "secs": 0.421,
      "rc": 0
    },
    {
      "id": "verify",
      "status": "completed",
      "conclusion": "failure",
      "fires": 1,
      "secs": 84.0,
      "rc": 101
    }
  ],
  "failed_events": ["verify.report.failed"]
}
```

| Field | Meaning |
| --- | --- |
| `schema` | `"peregrin/run-result/1"` — bump-gated contract tag |
| `run_id` | Stable per-run id — FNV-1a over campaign name + first event ts/id; identical wherever the same run's document is projected |
| `campaign` | `campaign.name` (`"(unnamed)"` when unset; the workflow file stem on a load failure) |
| `status` | Lifecycle: `completed` \| `cancelled` (`running` exists in the vocabulary but the artifact is only written at run end) |
| `verdict` | `pass` \| `fail` \| `error` — see below |
| `conclusion` | Canonical GH Checks outcome — see mapping below |
| `started_at` / `completed_at` | RFC3339 — the first/last event's `ts` (absent on a log-less error document) |
| `output` | GH Checks `output` block — `title` defaults to the campaign name; `summary`/`text` are `""` today (a future convention will populate them — the seam, not the machinery, ships now) |
| `nodes[]` | One entry per declared node (declaration order), plus any mid-run injected nodes sorted by id |
| `nodes[].status` | Lifecycle: `completed` — fired ≥once · `pending` — never fired |
| `nodes[].conclusion` | Outcome in the GH Checks vocabulary — absent while `pending` |
| `nodes[].fires` | Fire count — runner-projected results only (absent in supervisor-mode JSONL) |
| `nodes[].secs` | Summed `execute_node` wall time, millisecond-rounded — runner-projected only |
| `nodes[].rc` | Last numeric `payload.rc` the node emitted (the shell driver's exit-code contract) |
| `failed_events` | Event kinds that forced a non-`pass` verdict, deduplicated in first-seen log order |

## Declared `result:` rules

A campaign can declare event→conclusion mappings in its YAML —
optional, backward-compatible:

```yaml
result:
  - event: gate.pass
    conclusion: success
  - event: gate.fail
    conclusion: failure
```

Each rule asserts a conclusion when its `event` fires. Scanning
the log in order, **the last decisive event wins** — a declared
event asserts its rule's conclusion, an undeclared event falls
back to the heuristic below. Later events supersede earlier ones:
`gate.fail` then `gate.pass` concludes `success`.

## Conclusion semantics

The projection is a view over the run's event log (plus the
runner's fire stats) — no parallel tracking.

Per event, the decisive contribution is:

1. **A declared `result:` rule** for that event kind — its
   `conclusion` wins outright (even over the failure heuristics:
   declaring `node.failed → neutral` would soften it).
2. **The heuristic**, for undeclared kinds:
   - a `*.<failed>` terminal segment — `node.failed`,
     `workflow.failed`, and `emit:`-mapped kinds like
     `verify.report.failed` (the routed-type convention `ops runs`
     already reads);
   - `circuit.open`;
   - any event carrying a nonzero numeric `payload.rc` — a nonzero
     exit an `emit` map routed onto a *neutral* kind still fails;
   - `workflow.cancelled` → `conclusion: "cancelled"`, which also
     sets `status: "cancelled"` and dominates the run.

Run-level mapping:

| `verdict` | `conclusion` | Trigger |
| --- | --- | --- |
| `pass` | `success` | Last decisive event concluded success |
| `pass` | `neutral` / `skipped` | Only via declared rules / meta-only nodes |
| `fail` | `failure` | Failure-class heuristic or declared `failure`/`timed_out`/`action_required` |
| `fail` | `cancelled` | `workflow.cancelled` fired |
| `error` | `failure` | Campaign-plane `Err` — no GH `error` conclusion exists |

Note the deliberate gap between the engine's own `failed` flag and
this verdict: `has_failure` in the runner only watches
`node.failed`/`circuit.open`/`workflow.failed`, so a campaign whose
nodes emit `*.failed`-suffixed kinds still terminates
`workflow.done` — the run "succeeded" mechanically while a gate
failed. `RunResult` exists precisely to surface that: `verdict` is
`fail` even though the terminal event is `workflow.done`.

## Consumers

```bash
peregrin campaign run gate.yaml --history ci/gate.jsonl
# ci/gate.result.json now holds the verdict
jq -e '.conclusion == "success"' ci/gate.result.json
```

`verdict` remains the coarse gate (`pass`/`fail`/`error`); use
`conclusion` when you want the GH Checks vocabulary (e.g. posting
to a checks-compatible dashboard).

See [Gate a campaign run on its verdict](../how-to/gate-on-run-result.md)
for the CI recipe, and [`campaign run` in the CLI reference](cli.md#peregrin-campaign-run)
for the flag surface.
