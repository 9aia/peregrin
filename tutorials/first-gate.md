# Your first gate

> **Note:** the org's own CI gate is migrating back to bupha
> (`9aia/bupha//libraries/ci-gate`, `type: bupha`) — see org-ledger
> PL-013. This tutorial still applies to `peregrin-bundle` gates in
> general; the ci-gate bundle stays live until cutover.

This tutorial walks the operator flow end to end: install the
`ci-gate` bundle through tamias, deploy the shared campaign host,
register the repo webhook, and watch pushes turn into commit
statuses. You'll need the `peregrin`, `peregrin-hub`, and
`tamias-driver-peregrin` binaries
([Getting started](getting-started.md)), `tamias` itself, and a GitHub
token with `repo` + `admin:repo_hook` scopes.

Distribution, provisioning, and service lifecycle are **tamias**'s job —
peregrin ships only the provider (`tamias-driver-peregrin`). See
[Distribution](../reference/distribution.md) for the boundary.

The bundle: on a `push` event → detached worktree at the pushed sha
→ your gate command runs inside it → a GitHub commit status posts
under `ci/<name>`. Newest push on a `{repo}:{ref}` wins; a newer push
cancels the in-flight run.

## 1. Install the bundle

```bash
# `//` splits repo from subdir; `#` pins a ref (it defaults to the
# repo's default branch — this bundle lives on `v2`).
tamias install git:9aia/peregrin//libraries/ci-gate#v2
tamias list          # installed packages, resolved refs
```

## 2. Deploy it

```bash
tamias deploy ci-gate
```

`tamias deploy` does two things for a `peregrin-bundle`:

- renders `provider.services[]` into per-user systemd units. Every
  `peregrin-bundle` declares `provider.name: peregrin`, so they all
  share **one** unit — `peregrin-campaign.service` — whose
  `ExecStart` is `peregrin campaign serve
  ~/.config/peregrin/serve --listen 127.0.0.1:29173 --path /hook …`.
  One process hosts every deployed bundle on a single webhook
  listener, demuxing verified pushes by `repository.full_name`.
- runs `tamias-driver-peregrin deploy`, which generates the operator
  node config `~/.config/peregrin/ci-gate.yaml` — the bundle's
  `main.yaml` plus a `github-webhook` ingress node and the
  `capabilities.secrets` grant derived from the manifest's
  `provider.ingress[]` declaration — and drops a copy into the
  campaign-host dir `~/.config/peregrin/serve/ci-gate.yaml`.

The `capabilities.secrets` block is the grant the secret gate checks:
the webhook and status drivers may resolve exactly those names,
nothing else.

## 3. Fill the secrets

Secrets live in `~/.config/peregrin/env` (the unit loads it via
`EnvironmentFile`; create it `0o600` if absent). Config and bundles
carry a secret NAME, never its value — a name that doesn't resolve
fails the node rather than silently degrading:

```bash
GITHUB_TOKEN=ghp_…                 # forge token (statuses + clones + hooks)
PEREGRIN_WH_9AIA_PEREGRIN=…        # webhook secret, PEREGRIN_WH_<OWNER>_<REPO>
```

Ambient env also resolves — `export GITHUB_TOKEN=…` before a manual
`campaign run` beats the file (env precedes the file leg).

## 4. Check the box

```bash
tamias driver exec ci-gate health     # JSON probe: webhook ping / unit state
tamias driver exec ci-gate status     # per-unit is-active/is-enabled
tamias driver exec ci-gate logs       # journalctl over the provider units
peregrin campaign validate ~/.config/peregrin/ci-gate.yaml
```

Host diagnostics (`doctor`) live in tamias — `tamias doctor`. Fix
what they report before continuing.

## 5. Register the webhook

```bash
tamias driver exec ci-gate hooks register --dry-run   # print the plan first
tamias driver exec ci-gate hooks register
```

`hooks register` upserts the forge webhooks the manifest's
`provider.ingress[].hooks[]` declare — `ci-gate` ships `hooks: []`
(its `main.yaml` is a full `peregrin import bupha` artifact that owns
its ingress inline), so for this bundle the hooks already exist or are
registered by hand; bundles that *do* declare hooks get exact-URL
upserts here, and `hooks unregister` removes them.

The webhook listener binds `127.0.0.1:29173` on `/hook` (the shared
`--listen`/`--path` of `campaign serve`) — one socket serves every
installed `peregrin-bundle`. Point your tunnel or reverse proxy at
`https://ci.luisfuturist.com` (the manifest's `public_url`).

### Pull-request triggers (opt-in)

`github-webhook` verifies `pull_request` deliveries when a node sets
`params.pull_request_actions` (default `opened,synchronize,reopened`
— accepted actions outside the set are answered `200` as no-ops). A
`pull_request` event carries kind `pull_request` with payload fields
`repo`, `number`, `action`, `sha` (PR head), `ref` (head branch),
`base_ref`, `actor`, and `clone_url`. Checking out a fork's head
requires a token with read access to the fork repository.

## 6. Run the gate

```bash
systemctl --user enable --now peregrin-campaign.service
systemctl --user status peregrin-campaign.service
tamias driver exec ci-gate logs          # journalctl over the provider units
```

The unit's `ExecStart` is fixed by the shared provider — `campaign
serve` reads every `*.yaml` in `~/.config/peregrin/serve/`, so a
second `tamias deploy` of another bundle lands on the same listener
with no unit changes.

## 7. Push and watch

Every push to `owner/repo` on the gated ref flows: webhook →
verified → `push` event → worktree at the sha → `ci/<name>` status.

```bash
peregrin runs --history ~/.local/share/peregrin/serve-history/ci-gate.jsonl
peregrin runs --repo owner/repo --history ~/.local/share/peregrin/serve-history/ci-gate.jsonl
peregrin log -n 50 --history ~/.local/share/peregrin/serve-history/ci-gate.jsonl
peregrin explain 1a2b3c --history ~/.local/share/peregrin/serve-history/ci-gate.jsonl
```

`campaign serve --history-dir` appends one JSONL per campaign
(`<dir>/<campaign-name>.jsonl`); the observability verbs read any
`--history` log, including `campaign run --history` output. Statuses
are `done`, `failed`, `cancelled` (superseded), or `running`.

## 8. Tear down

```bash
tamias driver exec ci-gate hooks unregister   # if hooks were registered
tamias uninstall ci-gate                      # removes units + payload;
                                            # the driver drops the generated
                                            # configs (host dir + ~/.config)
```

## Wiring status

The ingress drivers (`github-webhook`, `sturn-poll`) are landed **and
wired**: `campaign run` and `campaign serve` register them against the
run's/shared `Injector`, wire the secret-resolution stores (keyring +
`~/.config/peregrin/env` behind the record and ambient env), and park
on the webhook channel — `campaign serve` shares ONE listener across
hosted campaigns. `peregrin campaign run --dry-run <config>` prints
the node graph without side effects; `campaign validate` checks the
wiring statically.

## Where next

- [Distribution](../reference/distribution.md) — tamias refs, stores,
  and the provider boundary
- [Fleet runners](../reference/fleet-runners.md) — `runs_on` labels and
  hub-side dispatch for campaign entities
- [Queue & supersede](../reference/queue-supersede.md) — the
  latest-wins + cancel-running policy the gate rides
- [Campaign schema](../reference/campaign-schema.md) — author your
  own bundle's `main.yaml`
- [Safety](../explanation/safety.md) — how `capabilities:` and the
  secret gate bound what a gate can do
