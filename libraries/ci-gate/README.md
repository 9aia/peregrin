# ci-gate

The bupha CI gate as a peregrin bundle (ADR-0080/ADR-0081): on a
`push` event → detached worktree at the pushed sha → inner `shell`
run → GitHub commit status under `ci/<campaign>`.

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
  *regenerates* that node config from `provider.ingress[]`: the
  `github-webhook` ingress node (`repos[]` from `hooks[]`,
  `secret_env` = `PEREGRIN_WH_<OWNER>_<REPO>`), `config.public_url`,
  and `capabilities.secrets` — **and** drops the same doc into the
  campaign-host dir `~/.config/peregrin/serve/ci-gate.yaml`.
- `provider.name: peregrin` is the **shared** provider identity
  (T8-7): every `peregrin-bundle` declares it, so all their `campaign`
  services render ONE unit — `peregrin-campaign.service` — whose
  `ExecStart` is `peregrin campaign serve ~/.config/peregrin/serve
  --listen 127.0.0.1:29173 --path /hook …`. `campaign serve` hosts
  every `*.yaml` in that dir — one generated config per deployed
  bundle — on a single webhook listener demuxed by
  `repository.full_name`. Shared-unit services must keep `exec`
  identical across bundles (last deploy wins the unit file). The unit
  is written + daemon-reloaded but **not** enabled; enabling and
  forge-hook registration stay operator steps.
- `tamias uninstall ci-gate` removes the units + store entry + both
  node configs; if another bundle still declares the shared unit,
  tamias re-renders it from the survivor instead of dropping it.

The bundle is the *policy* graph only. The node config (a workflow
document itself, per ADR-0080) is where the operator binds ingress and
secrets — `deploy` generates it; hand-editing is overwritten on the
next deploy (the manifest is the source of truth).

`provider.ingress[]` is declarative data for now: `public_url`,
`listen`, `path` and the watched `hooks[]` are recorded but nothing
registers live forge hooks yet — point your tunnel at
`listen`/`path` yourself (the pilot's `public_url` is owned by
bupha's tunnel). The full watched-repo set arrives with the PL-007 C2
`peregrin import bupha` import; the manifest declares the pilot's
`9aia/peregrin` `[push]` hook only.

Secrets: `~/.config/peregrin/env` (the unit's `EnvironmentFile`) must
carry `GITHUB_TOKEN` (forge token, `config.token_env`) and one
`PEREGRIN_WH_<OWNER>_<REPO>` per watched repo (the webhook HMAC
secret). Names only — values never ship in the bundle.

### Generated node config (what `deploy` writes)

```yaml
name: ci-gate
annotations:
  generated_by: tamias deploy (peregrin provider)
include:
  - <store>/ci-gate@<ver>/main.yaml
config:
  name: ci-gate
  public_url: https://ci.luisfuturist.com
capabilities:
  secrets: [GITHUB_TOKEN, PEREGRIN_WH_9AIA_PEREGRIN]
nodes:
  - id: ci-gate.ingress
    driver: github-webhook
    on: [start]
    emits: [ci-gate.ingress.listening, push]
    params:
      listen: 127.0.0.1:39271
      path: /hook
      repos:
        - name: 9aia/peregrin
          secret_env: PEREGRIN_WH_9AIA_PEREGRIN
```

`config:`/`capabilities:` merge *over* the included bundle document
(local wins per key, arrays concatenate), so bundle defaults
(`repo`, `ref`, `cmd`, `worktree_base`, `token_env`, `timeout`) still
apply unless the generated config overrides them. To gate a different
repo set or tune `cmd`, edit the manifest's `provider.ingress[]` +
`services[].exec`, or layer a hand-written overlay config and pass it
to `campaign serve` directly.

A `driver: sturn-poll` node works as the source instead of
`github-webhook` — declare it in a hand-written config (the generated
one only knows forge hooks).

### Legacy path

`peregrin provision deploy` no longer writes a systemd unit — it is
the same node-config generator the driver calls (now writing both
`~/.config/peregrin/ci-gate.yaml` and the host copy
`~/.config/peregrin/serve/ci-gate.yaml`). Pre-provider `ci-gate.service`
units keep working (`campaign run`/`serve` are unchanged); migrate by
`tamias deploy ci-gate` + enabling `peregrin-campaign.service`, then
`systemctl --user disable --now ci-gate.service`.

## Params (validated against `params.schema.json`)

| Key | Default | Meaning |
| --- | --- | --- |
| `name` | `ci-gate` | Status context base → `ci/<name>` |
| `repo` | `""` | `owner/name` to gate; empty gates all watched repos |
| `ref` | `refs/heads/main` | Ref to gate |
| `status_repo` | `""` | Post statuses to a different repo (bupha redirect) |
| `cmd` | `bash scripts/ci.sh` | Gate command, run inside the worktree |
| `worktree_base` | `~/.local/share/peregrin/worktrees` | `driver-git` `base` |
| `token_env` | `GITHUB_TOKEN` | Secret **name** for the forge token (clone + status) |
| `timeout` | `600` | Inner shell timeout, seconds |

Ingress params live on the webhook node (`repos[].name`,
`repos[].secret_env`, `listen`, `path`, `api_base`) — see
`docs/reference/` and `peregrin-driver-github-webhook`'s doc-comment.

## Event routing

`on:` matches event *types* only, so `ci-gate.match` re-emits a gated
push as `push.gated.<repo>.<clone_url>.<sha>` — fields ride the type and
downstream nodes recover them via `${event.match.1}`/`${event.match.2}`
(`**` re-joins clone_url's `.`-separated segments).

`node.failed` events carrying `{node, sha}` (driver-git
materialization failures) are re-mapped onto `<node>.failed.<sha>` by
`ci-gate.failmap` so a broken checkout posts a status (`state: error`)
instead of going silent.

Multi-repo gates: this campaign gates one `repo`+`ref` — for a fleet
of repos see `peregrin import bupha`, which emits per-repo routed
subscriptions (`push.<repo>.<ref>.<clone_url>.<sha>`) directly.
