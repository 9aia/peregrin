# Your first gate

This tutorial walks the operator flow end to end: install the
`ci-gate` bundle, materialize a node config from it, register the
repo webhook, run the gate as a service, and watch pushes turn into
commit statuses. You'll need the `peregrin` binary
([Getting started](getting-started.md)) and a GitHub token with
`repo` + `admin:repo_hook` scopes.

The bundle: on a `push` event → detached worktree at the pushed sha
→ your gate command runs inside it → a GitHub commit status posts
under `ci/<name>`. Newest push on a `{repo}:{ref}` wins; a newer push
cancels the in-flight run.

## 1. Install the bundle

```bash
# `//` splits repo from subdir; `#` pins a ref (it defaults to the
# repo's default branch — this bundle lives on `v2`).
peregrin install git:9aia/peregrin//libraries/ci-gate#v2
peregrin list          # name@version, resolved sha, source ref
peregrin info ci-gate  # manifest, provenance, size
```

The store lands at `~/.local/share/peregrin/store/ci-gate@<version>/`
— payload plus a `.peregrin-store.json` recording the source ref and
resolved commit sha. See
[Distribution](../reference/distribution.md) for the full ref
grammar (`#tag`, `#sha256:`, tarballs, local paths).

## 2. Materialize the config

```bash
peregrin create --template ci-gate \
  --repo owner/repo \
  --set cmd='bash scripts/ci.sh' \
  --set public_url=https://gate.example.com
```

`create` writes three things:

- **Node config** — `~/.config/peregrin/ci-gate.yaml`: includes the
  bundle's `main.yaml`, merges your `--set` values into `config:`,
  and adds the inline ingress node. Each `--repo owner/name[:ENV]`
  becomes a watched repo; the secret-env suffix names the webhook
  secret (default `PEREGRIN_WH_<OWNER>_<REPO>`).
- **Service unit** — `~/.config/systemd/user/ci-gate.service`,
  running `peregrin campaign run <config> --history
  $PEREGRIN_HOME/history.jsonl`. Written, *not* enabled.
- **Secrets skeleton** — `~/.config/peregrin/env` (created `0o600`
  if absent) listing every secret NAME the deployment references.

The `capabilities.secrets` block in the generated config is the grant
the secret gate checks: the webhook and status drivers may resolve
exactly those names, nothing else. Removing a name there revokes it.

```yaml
# ~/.config/peregrin/ci-gate.yaml — what create emits
name: ci-gate
annotations:
  generated_by: peregrin create --template ci-gate
  bundle: ci-gate@0.1.0
include:
  - /home/you/.local/share/peregrin/store/ci-gate@0.1.0/main.yaml
config:
  name: ci-gate
  cmd: bash scripts/ci.sh
  public_url: https://gate.example.com
capabilities:
  secrets:
    - GITHUB_TOKEN
    - PEREGRIN_WH_OWNER_REPO
nodes:
  - id: ci-gate.ingress
    driver: github-webhook
    on: [start]
    emits: [ci-gate.ingress.listening, push]
    params:
      repos:
        - name: owner/repo
          secret_env: PEREGRIN_WH_OWNER_REPO
```

## 3. Fill the secrets

Edit `~/.config/peregrin/env` — config and bundles carry a secret
NAME, never its value. The campaign resolves the name from this file
(the unit loads it via `EnvironmentFile`, so it *is* the environment
the run sees), and a name that doesn't resolve fails the node rather
than silently degrading:

```bash
GITHUB_TOKEN=ghp_…                 # forge token (statuses + clones + hooks)
PEREGRIN_WH_OWNER_REPO=…           # the webhook secret for owner/repo
```

## 4. Check the box

```bash
peregrin doctor --config ~/.config/peregrin/ci-gate.yaml
```

Doctor verifies the config loads, the dirs exist, the secret names
resolve, and the tools the campaign needs are on PATH. Fix what it
reports before continuing.

## 5. Register the webhook

```bash
peregrin deploy webhook --config ~/.config/peregrin/ci-gate.yaml
# owner/repo: created hook -> https://gate.example.com/hook
```

This reads every `github-webhook` node's `params.repos` and upserts
a `push`-events hook at `<public_url><hook-path>` via the GitHub API
— exact-URL match, so re-running is safe. `--url` overrides
`config.public_url`; `--token-env` overrides which secret name holds
the forge token. The webhook listener binds
`127.0.0.1:29173` on `/hook` by default (the `listen` param of the
`github-webhook` node) — change it when another service already holds
that port (a running bupha watcher does). Point your tunnel or
reverse proxy at it.

## 6. Run the gate

```bash
peregrin service enable --unit ci-gate   # start now + at login
peregrin service status --unit ci-gate
peregrin service logs --unit ci-gate -f
```

## 7. Push and watch

Every push to `owner/repo` on the gated ref flows: webhook →
verified → `push` event → worktree at the sha → `ci/<name>` status.

```bash
peregrin runs                    # newest-first: sha repo status wall events
peregrin runs --repo owner/repo  # one repo
peregrin log -n 50               # raw JSONL history tail
peregrin explain 1a2b3c          # why a sha did/didn't run — events + outcome
```

`runs` reads `$PEREGRIN_HOME/history.jsonl` — the durable log the
unit's `--history` flag appends to. Statuses are `done`, `failed`,
`cancelled` (superseded), or `running`.

## 8. Tear down

```bash
peregrin destroy webhook --all \
  --config ~/.config/peregrin/ci-gate.yaml   # delete the registered hooks
peregrin destroy service ci-gate             # disable + remove the unit
peregrin destroy worktrees \
  --config ~/.config/peregrin/ci-gate.yaml   # clean leftover run worktrees
```

## Wiring status

The ops verbs, the `ci-gate` bundle, and the `github-webhook`/
`sturn-poll` ingress drivers are landed **and wired**: `campaign run`
registers the ingress drivers against the run-scoped `Injector`,
populates `RunOptions.secrets`, and parks on the webhook channel.
`peregrin campaign run --dry-run <config>` prints the node graph and
the `<node>.listening` event without side effects; `validate` checks
the wiring statically. The observability verbs work on any
`--history` log, including `campaign run --history` output.

## Where next

- [Distribution](../reference/distribution.md) — `update`, `pin`,
  `verify`, the fetch cache
- [Queue & supersede](../reference/queue-supersede.md) — the
  latest-wins + cancel-running policy the gate rides
- [Campaign schema](../reference/campaign-schema.md) — author your
  own bundle's `main.yaml`
- [Safety](../explanation/safety.md) — how `capabilities:` and the
  secret gate bound what a gate can do
