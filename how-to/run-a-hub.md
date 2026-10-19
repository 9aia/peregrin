# Run a hub

This guide shows how to run `peregrin-hub` — the aggregation host that
merges nodes into one web UI and arbitrates session ownership.

## Basics

```bash
export PEREGRIN_NODES=http://node-a:8787,http://node-b:8787
export PEREGRIN_NODE_TOKEN=<node token>
peregrin-hub
```

Binds `PEREGRIN_HUB_HOST:PEREGRIN_HUB_PORT` — default `127.0.0.1:3000`.
Open it in a browser: the Leptos app loads, pairs with a node, and shows
the merged session view.

## What the hub does

- **Merges** — `GET /api/sessions` across every configured node into one
  list, tagging each row with its owning node.
- **Routes** — session ops go to the owning node; `peregrin-control`'s
  `pick_owner` resolves ownership when a session is visible from
  multiple nodes.
- **Arbitrates** — the lock registry decides attach outcomes: free →
  read-write, held → read-only, stale holder → preempt, node down →
  queue via the outbox.
- **Serves the app** — the wasm UI bundle (`PEREGRIN_SITE_ROOT`
  overrides the embedded assets).

## Auth on both sides

Two token hops exist and are independent:

- Browser → hub: `PEREGRIN_HUB_TOKEN` (empty = open on loopback).
- Hub → nodes: `PEREGRIN_NODE_TOKEN`, sent as bearer to every node in
  `PEREGRIN_NODES`.

## Credentials

API keys and node tokens live behind `peregrin-secret` —
`PEREGRIN_CREDENTIAL_STORE=keyring` (OS keychain, the default) or
`dotenv` (a `.env` file under `PEREGRIN_HOME`, useful in containers).
