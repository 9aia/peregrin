+++
id = "ADR-0092"
title = "Campaign driver: ops — bounded tamias-driver queries over tamias serve"
status = "open"
created = "2026-11-18"
owner = "devin"
priority = "med"
links = ["P-096"]
+++

# Campaign `driver: ops` — bounded tamias-driver queries over `tamias serve`

## Status

Proposed

## Context

Campaign yamls that need org tooling today fall back to `driver: shell`
wrappers — `cmd: "tamias-driver-lasius run forge pr merge …"`-style
shell+CLI plumbing with stringly params and `stdout` as the only result
channel. That breaks the typed-params/structured-payload contract every
other driver honors, and it bypasses the discovery/wire layer tamias
already owns (`tamias serve`: ndjson JSON-RPC `{"op":…}` →
`{"ok":…,"result"|"error"}` over stdio).

P-096 splits the work: tamias-serve gains `driver.*` ops
(`driver.list`/`driver.manifest`/`driver.query`/`driver.exec`) that
proxy installed `tamias-driver-<name>` binaries; peregrin gains a
generic campaign driver that speaks that wire. Domain expertise
(forge verbs, release verbs, ledger ops) stays in the domain repos —
neither side learns the other's vocabulary.

## Decision

New crate `peregrin-driver-ops` (`driver: ops`), registered in
`DefaultDriverRegistryLive` next to `shell`.

On node fire the driver:

1. resolves `params` through the standard template pass
   (`${event.*}`/`${params.*}`/`${secret.*}` — names only, values
   never serialized into run state beyond the emitted payload);
2. spawns `params.serve_argv` (default `["tamias","serve"]`; a bare
   `["tamias-serve"]` works where `tamias` isn't on PATH) under the
   sanitized `child_env` allowlist plus optional `params.env`;
3. writes ONE line `{"op":"driver.query","driver":…,"query":…,
   "params":{…},"timeout_ms":…}` and reads ONE response line;
4. kills the child (spawn-per-invoke v1 — pooled sessions are a
   follow-up; the sequential engine makes pooling a latency
   optimization only);
5. maps the envelope: outer `ok:true` → success emit with
   `payload.result` = the driver's query result (the pavo
   `QueryResult {query,data}` envelope — one envelope layer is
   unwrapped); outer `ok:false` → failure emit with the error text;
   spawn/timeout/I/O failure → the same failure path. Event kinds
   follow the `shell` convention: success → `emits[0]`/`ops.done`,
   failure → `emits[1]`/`node.failed`, both overridable via
   `params.emit` (`success`/`ok`/`true`, `failure`/`error`/`false`/
   `default`).

`params.echo` (array of dotted paths) copies values from the incoming
event's payload into the emitted payload at the same paths — the
payload-composition feature that replaces file-based state passing
between campaign nodes (`echo: ["number", "pull_request.head.sha"]`).

Scope is deliberately **bounded ops only**: `driver.query` targets
request/response queries. Ingress/stateful drivers (long-lived
`serve` surfaces, subscriptions) stay in-process campaign drivers —
`ops` is a worker node type, never an ingress one
(`Driver::is_ingress` stays false).

## Consequences

- Campaign yamls get typed params and structured results instead of
  shell quoting; `${secret.*}` still flows through `params.params`
  per the existing resolution chain.
- Every invoke pays one process spawn — acceptable for bounded ops;
  pooling is the documented follow-up if profiling says otherwise.
- `timeout` (seconds, default 120, `0` = none, cap 2h) is forwarded
  as `timeout_ms` so a well-behaved serve can fail cleanly before the
  kill; the kill is authoritative.
- The ops driver is wire-generic: any serve speaking the
  `{"op":"driver.query"}` contract works via `serve_argv` — tests
  stub it with `sh -c`, and non-tamias serves remain possible.
