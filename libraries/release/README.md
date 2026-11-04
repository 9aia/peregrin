# release

The release pipeline as a peregrin bundle — bupha's `release.sh` role:
push to the release ref → detached worktree at the pushed sha → the
release command (tag + `cargo publish` via `scripts/release.sh` or
equivalent) → GitHub commit status under `ci/<campaign>`.

Unlike `ci-gate`, `supersede.cancel_running` is **false**: a stale
queued release is pointless (latest-wins dedupes pending jobs), but a
half-published crate set is worse than a stale one — an in-flight
publish is never cancelled.

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
  cmd: "bash scripts/release.sh"
  token_env: "GITHUB_TOKEN"
capabilities:
  secrets: [CARGO_REGISTRY_TOKEN]
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
