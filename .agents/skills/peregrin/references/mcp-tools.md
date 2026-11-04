# peregrin-mcp — the agent tool surface

`peregrin-mcp` serves newline-delimited JSON-RPC over stdio — the
same hand-rolled MCP transport as `sturn-mcp` (no protocol crate).
Each `tools/call` returns `content[].text` carrying one JSON object;
failures set `isError: true` except `validate`, where an invalid
workflow is a valid `{ok:false}` answer.

Connect by spawning `peregrin-mcp` and speaking MCP on stdio
(`initialize` → `tools/list` → `tools/call`).

## Tools

| Tool | Params | Returns |
| --- | --- | --- |
| `campaign_run` | `path` *or* `bundle` (+ `workflow` inside it, default `main.yaml`); `config` obj, `secrets` obj (string→string), `dry_run` bool, `log` path, `limit` int | `{ok, workflow, events, terminal, tail[]}` |
| `validate` | `path` *or* `bundle`/`workflow` | `{ok, workflow, nodes, events, includes, warnings[]}` or `{ok:false, error}` |
| `bundle_install` | `ref` (required): `./path`, `/abs`, `git:<url>[//subdir][#rev]`, `https://….tgz[#sha256:…]` | `{label, name, version, source_ref, resolved_sha, digest, dir}` |
| `bundle_remove` | `name` (required) | `{removed, name, resolved_sha}` |
| `bundle_list` | — | `{bundles[]}` |
| `bundle_info` | `name` (required) | entry + `installed_at`, `size_bytes`, `description`, `include` |
| `runs` | `dir` (default `<peregrin-home>/runs`), `limit` | `{dir, runs[{path,events,sources,terminal,verdict}], total}` — recursive `*.jsonl` scan, newest first |
| `events` | `path` (required), `type`, `limit` | `{total, events[]}` — tail of the log |
| `explain` | `path` (required) | `{path, events, duration_ms, verdict, terminal, sources, kinds, failures[]}` |

Verdicts: `done` / `failed` / `cancelled` / `running` / `empty`.

## Notes for agents

- `campaign_run` `config` is a **top-level merge** over the
  campaign's `config:` map (a key replaces wholesale).
- `log` on `campaign_run` persists the JSONL log — feed that path to
  `events`/`explain`, or its parent dir to `runs`.
- `validate` before `campaign_run` — always. Wiring warnings
  (subscriptions nothing emits) are how half-broken graphs surface.
- Bundle `workflow` selects a non-`main.yaml` entry file inside an
  installed bundle.
- Advisory only: tools don't coordinate — `sturn claim` does.
