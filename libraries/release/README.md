# release

The release pipeline as a peregrin bundle — bupha's `release.sh` role:
push to the release ref → detached worktree at the pushed sha → the
release command → GitHub commit status under `ci/<campaign>`.

The default command is **`tarsi-serve release --yes`** — [9aia/tarsi](https://github.com/9aia/tarsi)'s
release expertise (workspace scan → conventional-commit bumps →
changelog → manifests → tags → crates.io/GitHub Releases → push),
driven one-shot. The released repo's `tarsi.toml` configures units,
tag format, and publish targets; any other shell pipeline still
works via the `cmd` param. `tarsi-serve` must be on PATH inside the
worktree environment (`cargo install --git ssh://git@github.com/9aia/tarsi.git tarsi-serve`).

Unlike `ci-gate`, `supersede.cancel_running` is **false**: a stale
queued release is pointless (latest-wins dedupes pending jobs), but a
half-published crate set is worse than a stale one — an in-flight
publish is never cancelled.

## Artifacts

The bundle is split into stdlib artifacts (ADR-0029) — `main.yaml`
carries the campaign-level concerns (`name`, `config`, `supersede`,
`annotations`, the `nodes:` wiring list) and each node lives in its
own `.node.yml`:

| File | Role |
| --- | --- |
| `match.node.yml` | Gate on `config.repo`/`config.ref`; re-emit `push.release.<clone_url>.<sha>`. |
| `pending.node.yml` | `pending` `ci/<name>` status on the gated sha. |
| `build.node.yml` | Detached worktree at the pushed sha → `config.cmd`. |
| `failmap.node.yml` | `node.failed` → `<node>.failed.<sha>` remap for the verdict. |
| `status.node.yml` | Verdict → `ci/<name>` commit status (success/failure/error). |
| `status-params.snippet.yml` | github-status params shared by `pending`/`status` (`snippet: status-params`). |
| `release.field.yml` | `config.*` surface as a handl field pattern (mirror of `params.schema.json`). |

`main.yaml`'s `nodes:` entries are bare `include:`s; the
`snippets:` entry declares `status-params` the same way. Loading
`main.yaml` as the top document also auto-discovers the sibling
artifacts — the explicit includes/`snippets:` keep the bundle intact
when `main.yaml` is itself `include:`d into another campaign.

## Node config

Same shape as `libraries/ci-gate` — an ingress node
(`driver: github-webhook` with `params.repos` + `secret_env` names, or
`driver: sturn-poll`) plus the `config:` keys in `params.schema.json`.
The inner env resolves `CARGO_REGISTRY_TOKEN` through the secrets
record — grant it in `capabilities.secrets` on the node config.

```yaml
config:
  name: release
  repo: "9aia/peregrin"
  ref: "refs/heads/main"
  cmd: "tarsi-serve release --yes"
  token_env: "GITHUB_TOKEN"
capabilities:
  secrets: [CARGO_REGISTRY_TOKEN, GITHUB_TOKEN]
nodes:
  - id: release.ingress
    driver: github-webhook
    on: [start]
    params:
      repos:
        - name: "9aia/peregrin"
          secret_env: "BUPHA_WH_9AIA_PEREGRIN"
```

## Event routing

`push.release.<clone_url>.<sha>` is the gated type
(`${event.match.1}`/`match.2` recover url + sha; `**` re-joins
clone_url's segments). See `libraries/ci-gate/README.md` for the full
mechanics — this bundle is the same graph with release-specific
defaults and no run cancellation.
