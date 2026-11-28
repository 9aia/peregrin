# peregrin-node

The peregrin **node daemon** as a tamias-deployed service — the
counterpart to the campaign bundles (`ci-gate`, `docs-deploy`).
Where those host workflows on a webhook listener, this one runs
`peregrin serve`: the session plane, entity API, and campaign
engine on `PEREGRIN_HOST:PEREGRIN_PORT` (default
`127.0.0.1:18787`).

`type: peregrin-node` is the marker: the bundle ships **no**
`main.yaml` campaign payload, and `tamias-driver-peregrin deploy`
skips node-config generation for it — `provider.services[]` is the
whole payload, rendered by `tamias deploy` into
`peregrin-node.service` (`provider.name: peregrin` + service
`node`).

## Install + deploy

```sh
tamias install ./libraries/peregrin-node    # or a git:/http: ref
tamias deploy peregrin-node                 # → peregrin-node.service
systemctl --user enable --now peregrin-node.service
```

- `~/.config/peregrin/env` is the unit's `EnvironmentFile` —
  `PEREGRIN_HOST`/`PEREGRIN_PORT`/`PEREGRIN_TOKEN` and the
  `PEREGRIN_*` tuning knobs live there (names only, values never
  ship in the bundle).
- **ci-job durability:** set `PEREGRIN_SNAPSHOT_TARGET` to the hub
  base URL (e.g. `http://127.0.0.1:13000`),
  `PEREGRIN_SNAPSHOT_TOKEN` to the hub bearer
  (`PEREGRIN_HUB_TOKEN`), and `PEREGRIN_SNAPSHOT_NODE` to this
  node's `PEREGRIN_NODES` registry id (e.g. `main`) — hub
  entity-index buckets key on that id, not the node's self id.
  Without the target/token the node's `kind="ci-job"` plane boots
  empty on every restart — the plane is in-memory and rehydrates
  only from the hub's entity index. Same vars the ephemeral
  shutdown flush uses; see
  `docs/how-to/run-ci-through-ci-job-entities.md`.
- `tamias driver exec peregrin-node status|logs|health` probes the
  rendered unit (`is-active`/`journalctl` — there is no ingress
  HTTP probe).
- `tamias uninstall peregrin-node` removes the unit + store entry.

## Migrating a hand-managed unit

A hand-written `peregrin.service` (or `peregrin-node.service`)
running `peregrin serve` is equivalent — migrate by
`tamias deploy peregrin-node`, enable the rendered unit, then
`systemctl --user disable --now <old-unit>`.
