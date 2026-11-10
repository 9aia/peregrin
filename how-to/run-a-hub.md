# Run a hub

This guide shows how to run `peregrin-hub` — the headless aggregation
gateway that merges nodes into one API and arbitrates entity/session
ownership across the fleet.

## Basics

```bash
export PEREGRIN_NODES="durable=http://127.0.0.1:18787@tok1;ci=http://10.0.0.5:18787@tok2"
export PEREGRIN_NODE_TOKEN=<node token>
peregrin-hub
```

`PEREGRIN_NODES` is `id=url[@token]` pairs, semicolon-separated — the
`id` is how the hub names the node everywhere (entity rows, dispatch,
`?node=` selectors). It maps onto Hypha's `HYPHA_NODES`; `peregrin-hub`
reads only `PEREGRIN_NODES` (`HYPHA_NODES` applies to the standalone
`hypha-hub` binary).

Binds `PEREGRIN_HUB_HOST:PEREGRIN_HUB_PORT` — default
`127.0.0.1:13000` (`PEREGRIN_HUB_PORT` falls back to `PEREGRIN_PORT`).
Non-loopback without `PEREGRIN_HUB_TOKEN` refuses to start.

## What the hub does

- **Merges** — the generic `/api/entities*` surface spans every
  configured node; `GET /api/node-status` reports per-node health.
- **Routes** — session ops go to the owning node; `hypha-control`'s
  `pick_owner` resolves ownership when an entity is visible from
  multiple nodes.
- **Arbitrates** — the lock registry decides attach outcomes: free →
  read-write, held → read-only, stale holder → preempt, node down →
  queue via the durable outbox (`GET /api/pending-ops` shows its
  depth).
- **Dispatches** — hub-side fleet dispatch: a `kind="campaign"`
  entity's `campaign.start` op is routed to a node whose
  `PEREGRIN_RUNNER_LABELS` satisfy the campaign's `runs_on` (durable
  nodes preferred over ephemeral, id tiebreak). See
  [Fleet runners](../reference/fleet-runners.md).

There is no bundled web UI on `main` — the hub serves the API only.
The experimental Leptos UI lives on the `experimental/web` branch.

## Auth on both sides

Two token hops exist and are independent:

- Client → hub: `PEREGRIN_HUB_TOKEN` (empty = open on loopback).
- Hub → nodes: the per-node `@token` in `PEREGRIN_NODES`, sent as
  bearer; `PEREGRIN_NODE_TOKEN` seeds nodes listed without one.

`/api/pair` trades a one-time code minted on a node for a bearer
credential (`sepia_…`), and `/api/auth/login`/`logout` run the cookie
flow — `?access_token=` works only on the SSE paths.

## Credentials

Secrets sit behind `hypha-secret`, mapped through the `PEREGRIN_*` →
`HYPHA_*` env boundary. `PEREGRIN_CREDENTIAL_STORE` accepts `auto`
(default: OS keyring, file fallback), `keyring`, or `file` — the file
store lives under the Hypha home
(`$HYPHA_HOME/hub/credentials.json` or the mapped Peregrin home), not a
`.env` dotenv backend.
