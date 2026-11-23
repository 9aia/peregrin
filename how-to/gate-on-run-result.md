# Gate a campaign run on its verdict

CI gates need a machine-readable pass/fail — `campaign run`'s exit
code stays `0` on a completed run by design, so the verdict lives in
the **`RunResult`** artifact ([reference](../reference/run-result.md)).

## One-shot gate

```bash
peregrin campaign run gate.yaml --result ci/result.json
jq -e '.verdict == "pass"' ci/result.json
```

Or let `--history` imply the sibling — useful when the same job also
wants the durable event log for `peregrin runs`/`explain`/`log`:

```bash
peregrin campaign run gate.yaml --history ci/gate.jsonl
# writes ci/gate.result.json automatically
jq -e '.verdict == "pass"' ci/gate.result.json
```

## Reading the verdict

The document splits **lifecycle** from **outcome**:

- `status` — `completed` | `cancelled` (`running` exists in the
  vocabulary but the artifact is only written at run end).
- `conclusion` — the GH Checks vocabulary:
  `success` | `failure` | `neutral` | `skipped` | `timed_out` |
  `cancelled` | `action_required`.
- `verdict` — the campaign's coarse label: `pass` | `fail` | `error`.
  `error` (campaign-plane failure) maps to `conclusion: "failure"`.

Per-node detail for flake triage:

```bash
jq '.nodes[] | select(.status != "completed" or .conclusion == "failure")' ci/result.json
```

## Declaring your own verdicts

When a campaign's gates speak in custom event kinds, declare the
mapping in the YAML — last decisive event wins:

```yaml
result:
  - event: gate.pass
    conclusion: success
  - event: gate.fail
    conclusion: failure
```

Undeclared events keep the heuristic: `*.failed` kinds,
`circuit.open`, nonzero `payload.rc`, and `workflow.cancelled`
still conclude `failure`/`cancelled`.

## Continuous campaigns

A `supersede:` campaign with ingress nodes runs supervised — each
trigger spawns a run, so there is no single verdict. The same
`--result`/`--history`-derived path is appended to as **JSONL**, one
`RunResult` per finished run:

```bash
jq -s 'map(select(.verdict == "fail")) | length' ci/gate.result.json
```

`run_id` is stable per run — the same document reads identically
wherever it's projected, so you can correlate result lines with
`peregrin runs` history.

## Notes

- `--dry-run`/`--replay` runs write the artifact too — a dry run's
  `rc`/`secs` reflect the simulated fire.
- The artifact is written even when the engine returns `Err`
  (verdict `error`, conclusion `failure`) — a CI job that ignores
  the exit code still gets the document.
- `output.title` is the campaign name; `output.summary`/`text` are
  empty today — a future convention will populate them for
  checks-dashboard rendering.
