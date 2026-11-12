# tamias-driver-peregrin — the agent ops surface

The driver binary serves ndjson JSON-RPC on stdio (`serve`) — pavo's
view/ops wire, no protocol crate. Spawn
`tamias-driver-peregrin serve` (or reach it through pavo discovery)
and speak `{"op": …}` requests:

- `{"op":"ops"}` / the `ops` argv verb — the ops manifest as JSON.
- `{"op":"query","query":"cli.<verb>","params":{…}}` — run one
  bounded `peregrin` verb; params map onto the verb's clap args.
- `{"op":"manifest"}`, `{"op":"views"}`, `{"op":"view",…}` — the
  pavo view half (read-only store/cache inspection).

`tamias-driver-peregrin run <peregrin argv>` is the passthrough —
each `cli.*` query re-execs it in a captured child, so engine output
never touches the serve stream. `cli <argv>` is the `bins[]` shim
verb behind the `peregrin` command itself.

## Ops manifest

Derived from `peregrin_cli::cli::command()` — every bounded verb is
an op `cli.<dotted.path>` (`cli.sessions.list`, `cli.runs`,
`cli.version`, …). Each op entry:

- `params[]` — `{name, kind, doc}`; `kind` is `arg` (required
  positional), `opt` (`--name <v>` or optional positional), `flag`
  (`--name`). Spell params by flag name; arrays flatten to repeated
  flags. `--token`/`--source-token` carry `"secret": true` — pass
  via params-stdin.
- `mutates` — `false` only for the read-only set (probes, history
  readers, `campaign.validate`, list/export verbs). Treat `true`
  ops as writes.

Remote verbs (`sessions`, `projects`, `config`, `push`, `health`,
`node`, …) take `--node`/`--token` params; env
(`PEREGRIN_NODE_URL`/`PEREGRIN_TOKEN`) applies when absent. With no
node answering, the child exits `code: 1` and the transport error
lands on `stderr` — honest data, not a wire failure.

## Surfaces — never ops

`serve`, `campaign.run`, `campaign.serve`, `sessions.attach`,
`sessions.stream`, `events` are listed under `surfaces` in the
manifest: they never return or need inherited stdio, so they are
reachable only via `run`/`cli` passthrough (or a streaming surface
driver — the optional `mcp` feature exposes ops as MCP tools on
stdio). The hidden `peregrin provision` plumbing is skipped too —
it stays reachable through `run`.

## Notes for agents

- Query `ops` first — the manifest is the contract; don't hardcode
  verb lists (new bounded verbs appear automatically).
- `campaign run`/`serve` are surfaces: to *execute* a campaign use
  `cli.campaign.submit` (returns `{run_id, log}` immediately) then
  poll `cli.campaign.status` (`verdict`: `pending`/`running`/
  `done`/`failed`/`cancelled`) and `cli.campaign.events` (bounded
  `--limit` tail, `--type` filter). `cli.campaign.validate` is the
  bounded check (`--bundle`/`--workflow` select inside an installed
  bundle). The hidden `campaign exec` verb is the submit child's
  body — never an op.
- Run logs are JSONL in two families: push-run history
  (`cli.runs`/`cli.log`/`cli.explain` take `--history <path>`,
  default `$PEREGRIN_HOME/history.jsonl`) and campaign run logs
  (`cli.logs.runs` scans a dir recursively — default
  `$PEREGRIN_HOME/runs` — `cli.logs.events`/`cli.logs.explain` take
  a log path; `cli.campaign.events`/`status` resolve `<run-id>` to
  `$PEREGRIN_HOME/runs/<run-id>.jsonl`).
- Bundle lifecycle delegates to the tamias pipeline + store:
  `cli.bundle.install|remove|list|info` (`bundle install` takes
  `--allow-drivers` for the bundled-driver consent gate) resolve
  `$TAMIAS_HOME/store`/`$TAMIAS_CACHE` — the same entries
  `tamias install`/`list`/`info`/`remove` see.
- Advisory only: ops don't coordinate — `sturn claim` does.
