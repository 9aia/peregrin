# The substrate model

Peregrin is converging on being the 9aia factory's single event
substrate: triggers in, execution out, campaigns as policy between
them (ADR-0080). Everything below is load-bearing — get it right
before designing anything on top.

## One grammar

A campaign is a YAML file with `name:`/`config:`/`nodes:` and the
fields in `docs/reference/campaign-schema.md`. A **node config file**
(the daemon's `peregrin.yaml`) is the same grammar — it just contains
*ingress driver nodes* (`driver: github-webhook`, `driver:
sturn-poll`) whose params carry the deployment-specific bits. There
is no separate triggers/campaigns dialect.

Nodes: `id`, `driver`, `handler`, `on: [glob]`, `emits: [type]`,
`mode: any|all`, `params`, `max_fires`, `until`, `retire_on`,
`retry`, `circuit_breaker`, `poll`, `fan_in`, `forward`, `state`,
`capabilities`, `input`/`output` contracts.

## Events

Every unit the bus carries is the closed envelope `{type, payload,
source, id, ts}` (`docs/reference/event-envelope.md`). The engine
synthesizes `start`, `workflow.done`, `workflow.failed`,
`workflow.cancelled`, `node.failed`, `node.exhausted`, `node.skipped`,
`event.invalid`, `capability.violation`, `circuit.open`,
`<node>.tick`. Nothing secret ever rides the envelope — secrets reach
nodes through the run's secrets record (`ctx.secrets`,
`capabilities.secrets`, `${secret.*}`).

## The run

`peregrin_node::campaign` composes the plane in-process: driver
registry (agent/http/human/shell/git/github-status/noop/runner),
`LocalBus`, memory state + DLQ, sequential `Runner`. `run(path,
options)` is synchronous and returns the event log whose last entry
is terminal. `runner_with_log(log_path)` swaps in the durable JSONL
store — the log survives exit and is what `runs`/`events`/`explain`
read.

## Bundles

A bundle is a payload dir with a `tamias.yaml` manifest (`name`,
`version`, `type: peregrin-bundle`, optional `provider:` block,
`include:`). Distribution is **tamias**'s: `tamias install <ref>`
resolves `./path` / `git:<url>[//subdir][#rev]` / `http(s).tgz` /
`cf:` refs, fetches into the content-addressed fetch cache
(`~/.cache/tamias/fetch`), and unpacks into the install store
(`~/.local/share/tamias/<name>@<version>/`) recording source ref,
resolved sha, and payload digest. Manifest `include:` entries are
store names or installable refs; `libraries/` is the in-repo stdlib
of fragments (`retry`, `dead-letter`, `circuit-breaker`, `ci-gate`,
`release`, `display`, `git`, `agent`, `common`, `test`, `peregrin`).
Deploy/service lifecycle is `tamias deploy` + `tamias-driver-peregrin`'s
provider verbs — see `docs/reference/distribution.md`.

## Scope resolution

`${config.*}`, `${event.*}`, `${params.*}`, `${state.*}`,
`${node.*}`, `${secret.*}` — see `docs/reference/scope-resolution.md`.
`config` merges over the campaign `config:` map; `secrets` populate
the run record and are capability-gated.
