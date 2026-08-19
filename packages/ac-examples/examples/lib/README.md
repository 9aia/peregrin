# examples/lib — Reusable node YAMLs

Canonical, copyable node definitions extracted from real campaigns. These
are **data, not code** — new capabilities as YAML entries, per the project's
thesis. Copy any of these into your project's `.ac/lib/` (or alongside your
workflow) and the file router auto-discovers them.

## What's here

### git/ — git operations via `driver: shell`

- `worktree.node.yml` — create a git worktree, stores path/branch in `${state}`
- `commit.node.yml` — stage and commit with a conventional message
- `push.node.yml` — push current branch to origin
- `pr.node.yml` — create a GitHub PR via `gh`

### agent/ — AI agent invocation via `driver: agent`

- `investigate.node.yml` — normal-mode agent for root cause analysis
- `fix.node.yml` — agent-driven fix with retry loop (`max_fires` + `${node.fires}`)
- `review.node.yml` — normal-mode agent for code review

### test/ — test runner steps via `driver: shell`

- `install.node.yml` — install dependencies
- `check.node.yml` — lint/typecheck
- `run.node.yml` — run targeted tests

### campaign/ — multi-PR campaign primitives

- `merge-gate.node.yml` — HITL merge confirmation (`driver: human`)
- `wave-fanin.node.yml` — declarative fan-in for parallel PR waves (`fan_in:`)
- `shard-dispatcher.node.yml` — fan out start events for parallel shard pipelines

## How to use

Copy the files you need into your project:

```bash
cp examples/lib/git/*.node.yml .ac/lib/git/
cp examples/lib/agent/*.node.yml .ac/lib/agent/
```

Then reference them from your workflow. The file router auto-discovers
`*.node.yml` files in `.ac/` and alongside the workflow.

## What these replace

These nodes were extracted from real campaigns that used workarounds:

| Workaround                                | Replaced by                                 |
| ----------------------------------------- | ------------------------------------------- |
| `/tmp/ac-<name>-env` sidecar files        | `${state.worktree_path}`, `${state.branch}` |
| `event.type.split('.')[1]`                | `${event.shard}`                            |
| `/tmp/ac-<name>-attempt` counter          | `${node.fires}`                             |
| `envsubst` + `devin -p` shell boilerplate | `driver: agent` with `prompt_file`          |
| `read -r RESPONSE` merge gates            | `driver: human` with `expect`               |
| Per-wave `wave_fanin` Python handler      | `fan_in: {count_from: ...}`                 |
| Thin orchestrator `echo` bridge nodes     | `forward: [event.type]`                     |
