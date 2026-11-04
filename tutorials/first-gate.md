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
peregrin install git:9aia/peregrin//libraries/ci-gate
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

Edit `~/.config/peregrin/env` — names only ever resolve through
environment → OS keyring (service `peregrin`) → this file, never out
of config:

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
the forge token. The webhook listener itself binds
`127.0.0.1:29173` on `/hook` by default — point your tunnel or
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
`sturn-poll` ingress drivers are all landed. The last link —
registering the ingress drivers against the node's per-run
`Injector` and feeding `RunOptions.secrets` from the secret chain —
is in flight: until it lands, `peregrin campaign run` on a webhook
config reports `no driver registered for 'github-webhook'` and the
unit's `Restart=on-failure` cycles it harmlessly. Everything through step 5 (install,
materialize, secrets, hook registration) is real today; the
observability verbs work on any `--history` log, including
`campaign run --history` output.

## Where next

- [Distribution](../reference/distribution.md) — `update`, `pin`,
  `verify`, the fetch cache
- [Queue & supersede](../reference/queue-supersede.md) — the
  latest-wins + cancel-running policy the gate rides
- [Campaign schema](../reference/campaign-schema.md) — author your
  own bundle's `main.yaml`
- [Safety](../explanation/safety.md) — how `capabilities:` and the
  secret gate bound what a gate can do
