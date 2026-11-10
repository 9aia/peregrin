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
- `tamias driver exec peregrin-node status|logs|health` probes the
  rendered unit (`is-active`/`journalctl` — there is no ingress
  HTTP probe).
- `tamias uninstall peregrin-node` removes the unit + store entry.

## Migrating a hand-managed unit

A hand-written `peregrin.service` (or `peregrin-node.service`)
running `peregrin serve` is equivalent — migrate by
`tamias deploy peregrin-node`, enable the rendered unit, then
`systemctl --user disable --now <old-unit>`.
