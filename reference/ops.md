# Engine ops surface

`tamias-driver-peregrin` is a **dual-surface** binary: the pavo
view/ops driver *and* the tamias content-type driver for
`peregrin-bundle`/`peregrin-node` packages. The engine half exposes
the whole `peregrin` CLI to agents and hosts without shelling out —
the ops manifest is **derived from the engine's own clap tree**
(`peregrin_cli::cli::command`), so a verb can't exist on the CLI and
be missing from the wire surface.

## Surfaces

| Verb | What it does |
| --- | --- |
| `manifest` | Print the driver manifest JSON (pavo discovery probe) |
| `capabilities` | Merged pavo + tamias capability payload (`types`, `verbs`) |
| `serve` | ndjson JSON-RPC on stdio — `{"op":"ops"}` answers the ops manifest; `{"op":"query","query":"cli.<verb>","params":{…}}` runs one bounded verb |
| `ops` | The ops manifest as JSON, standalone |
| `run <argv>` | Engine passthrough — `tamias-driver-peregrin run version` ≡ `peregrin version` (also the child side of every `cli.*` query) |
| `cli <argv>` | The tamias `bins[]` shim verb — the `peregrin` command itself; strips the standard `--dir`/`--name` driver flags and runs the rest through `peregrin_cli::cli::run_from` with inherited stdio |

## `cli.<verb>` ops

Every **bounded** CLI verb is an op named `cli.<dotted.path>` —
`cli.sessions.list`, `cli.campaign.validate`, `cli.version`, … Each
op carries:

- `params` — typed entries derived from the clap args
  (`arg` = required positional, `opt` = `--name <v>`/optional
  positional, `flag` = `--name`). Remote verbs take the node target
  as ordinary params (`--node`, `--token`, `--agent`); env defaults
  (`PEREGRIN_NODE_URL`/`PEREGRIN_TOKEN`) apply when they're absent.
  `--token`/`--source-token` are marked `secret` — pass them via
  params-stdin, not argv.
- `mutates` — `false` for the read-only set (probes, history readers,
  `campaign.validate`, list/export verbs); `true` for everything that
  writes node state, local files, or executes workflows.

A `cli.*` query re-execs `run <argv>` in a captured child and returns
`{op, argv, code, stdout, stderr}` — a nonzero `code` is the engine's
own exit status (e.g. `1` + the transport error on `stderr` when no
node answers), not a wire failure.

## Surfaces, not ops

Unbounded or host-interactive verbs can never be request/response —
they never return or need inherited stdio. They ride `surfaces`
instead and are reachable only through `run`/`cli` passthrough or a
streaming surface driver:

- `serve` — the node daemon itself
- `campaign.run` / `campaign.serve` — campaign execution and the
  multi-campaign webhook listener
- `sessions.attach` — interactive live control
- `sessions.stream` / `events` — live event streams

## MCP

The driver's optional `mcp` cargo feature embeds the pavo-driver-kit
surface layer, exposing the same ops manifest as MCP tools on stdio —
equivalent to the standalone `tamias-driver-mcp` surface driver minus
the PATH/discovery hop. The hand-rolled `peregrin-mcp` server is
**removed**; MCP access, where needed, comes through this generated
surface, not a peregrin-owned binary.

## See also

- [HTTP API](api.md) — the `/api/*` surface remote ops hit
- [Distribution](distribution.md) — the tamias verbs around the driver
- [Queue & supersede](queue-supersede.md) — run semantics `campaign.*` gets
