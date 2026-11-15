# Boundaries

What peregrin owns — and the neighbors it deliberately doesn't.

## Owns

- The **integration side** of the entity plane: sessions as
  `kind="sepia-session"` entities, campaigns as `kind="campaign"`
  entities, namespaced ops, campaign hosting (`campaign serve`),
  fleet scheduling via `runs_on`.
- The **orchestration plane**: campaigns, drivers, events, templates,
  loaders, stores, remote transport, the outbox for offline ordering.
- The **policy decision** over sessions: who may act, when, and where
  (locks, takeover arbitration, hub-side routing).
- `peregrin-http`'s composition of session routes with the generic
  substrate routes under one listener.

## Doesn't own

| Neighbor | Owns instead |
| --- | --- |
| [sepia](https://9aia.com/sepia/) | Session mechanism — attach, prompt, restore, per-agent stores. Peregrin spawns `sepia serve` and never links `sepia-session` into orchestration code. |
| [hypha](https://github.com/9aia/hypha) | The generic substrate — entity plane, projection, outbox, locks, sync, the `/api/entities*` + generic HTTP routes. Consumed as git deps. |
| [tamias](https://9aia.com/tamias/) | Bundle distribution, provisioning, deploy, service lifecycle. Peregrin ships only the `tamias-driver-peregrin` provider; the `peregrin` command itself is a tamias `bins[]` shim. |
| [sturn](https://9aia.com/sturn/) | Ephemeral coordination — claims/ops/topics. Peregrin consumes it via the `sturn` driver and `sturn-poll` ingress; it doesn't own the WAL. |
| [tarsi](https://9aia.com/tarsi/) | The release plane — `libraries/release` drives `tarsi-serve` ops; peregrin doesn't cut releases. |

The web UI (`peregrin-web*`) is experimental and lives on the
`experimental/web` branch — `main` is headless.
