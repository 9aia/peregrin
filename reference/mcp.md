# MCP server

`peregrin-mcp` is an MCP stdio server — newline-delimited JSON-RPC —
that exposes the campaign plane and bundle verbs to agents. Every
tool is a thin wrapper over the same library composition the
`peregrin` CLI drives. The server is **advisory**: coordination and
claims belong to [sturn](https://github.com/9aia/sturn), not to MCP
tool calls.

## Tools

| Tool | What it does |
| --- | --- |
| `campaign_run` | Load a workflow file (or an installed bundle's `main.yaml`) and run it on this machine's campaign plane |
| `validate` | Load + validate a workflow file — parse, expansion, normalization errors plus wiring warnings |
| `bundle_install` | Install a bundle ref (`./path`, `git:<url>[//subdir][#rev]`, `http(s)` `.tgz`) into the store |
| `bundle_remove` | Drop an installed bundle's store entry (fetch cache untouched) |
| `bundle_list` | List installed bundles — `name@version`, resolved sha, source ref |
| `bundle_info` | A bundle's manifest, provenance, and size |
| `runs` | Scan a dir for JSONL run logs and summarize each |
| `events` | Tail events from a JSONL run log, optionally filtered by type |
| `explain` | Summarize one run log — verdict, per-source counts, failures, duration |

## Context

`Ctx` derives its roots from `PEREGRIN_HOME`/`PEREGRIN_CACHE`/XDG —
the same store/cache layout the CLI's distribution env uses. The
`bundle_*` tools currently drive the pinned pre-provider tamias
deps directly (fs/git/http sources + local store); they are the MCP
surface's own copy of the install pipeline, not a `tamias` CLI
shell-out.

## See also

- [Campaign schema](campaign-schema.md) — what `validate`/`campaign_run` load
- [Queue & supersede](queue-supersede.md) — run semantics `campaign_run` gets
- [Distribution](distribution.md) — the ref grammar `bundle_install` accepts
