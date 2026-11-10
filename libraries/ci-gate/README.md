# ci-gate

The org CI gate as a peregrin bundle: on a `push` event → detached
worktree at the pushed sha → per-repo job → GitHub commit status.
This is the **fleet artifact** — `main.yaml` is the full
`peregrin import bupha` output (188 repos) with the live hand-tuning
applied (inner `timeout: 600`, `~/.local/share/peregrin/worktrees`
base, `capabilities.secrets`). It is self-contained: its own
`github-webhook` ingress node, per-repo cmds, status contexts
(`ci/bupha*`) and `secret_env` names are all inline.

It is deliberately NOT the ADR-0080 template form (one `config.cmd`
for every repo): the imported fleet is heterogeneous — per-repo
gate scripts (`scripts/ci.sh`, `gates.sh`, `ci/trigger.sh`,
`scripts/release.sh`, `deploy-docs.sh`), per-repo status contexts,
and two secret schemes (`BUPHA_WH_<REPO>` + shared
`BUPHA_WEBHOOK_SECRET`). A single-params template cannot express
that; the bundle ships the concrete campaign instead (same pattern
as `docs-deploy`).

## Install + deploy (tamias provider flow)

`type: peregrin-bundle` + the `provider:` block in `tamias.yaml` make
this a declarative tamias package:

```sh
tamias install ./libraries/ci-gate          # or a git:/http: ref
tamias deploy ci-gate                       # → peregrin-campaign.service
systemctl --user enable --now peregrin-campaign.service
```

- `tamias install` dispatches `tamias-driver-peregrin install`, which
  writes a bare `~/.config/peregrin/ci-gate.yaml` (`include:` of the
  bundle's `main.yaml`).
- `tamias deploy` dispatches the driver's `deploy` verb, which
  regenerates that node config — with `hooks: []` it is a pure
  `include:` + `config:` doc — **and** drops the same doc into the
  campaign-host dir `~/.config/peregrin/serve/ci-gate.yaml`.
- `provider.name: peregrin` is the **shared** provider identity
  (T8-7): every `peregrin-bundle` declares it, so all their `campaign`
  services render ONE unit — `peregrin-campaign.service` — whose
  `ExecStart` is `peregrin campaign serve ~/.config/peregrin/serve
  --listen 127.0.0.1:29173 --path /hook …`. `campaign serve` hosts
  every `*.yaml` in that dir — one generated config per deployed
  bundle — on a single webhook listener demuxed by
  `repository.full_name`. The unit is written + daemon-reloaded but
  **not** enabled; enabling stays an operator step.
- `tamias uninstall ci-gate` removes the units + store entry + both
  node configs; if another bundle still declares the shared unit,
  tamias re-renders it from the survivor instead of dropping it.

## Secrets

`~/.config/peregrin/env` (the unit's `EnvironmentFile`) must carry
`GITHUB_TOKEN` and every `secret_env` name the campaign references —
`BUPHA_WH_<OWNER>_<REPO>` per repo plus the shared
`BUPHA_WEBHOOK_SECRET`. Names only — values never ship in the
bundle. (The `PEREGRIN_WH_*` convention applies only to
manifest-declared `hooks[]`, which this bundle deliberately leaves
empty.)

## Regenerating

The fleet membership/cmds come from the bupha import. To re-import
after bupha config changes: `peregrin import bupha
~/.config/bupha/bupha.yaml` (bupha is archived — this file is now
the source of truth), apply the same hand-tuning (timeout 600,
worktree base, capabilities.secrets), and bump `version:`.

## Legacy path

A hand-managed `ci-gate.service` running
`peregrin campaign serve ~/.config/peregrin/serve` is equivalent —
migrate by `tamias deploy ci-gate` + enabling
`peregrin-campaign.service`, then `systemctl --user disable --now
ci-gate.service`.
