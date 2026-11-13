# Distribution — bundles, tamias, the provider boundary

A *bundle* is a manifest (`tamias.yaml` / `.yml` / `.toml`) plus a
payload of workflows, `.node.yml`, `.driver.yml` and `.field.yml`
files — the `peregrin-bundle` content type. **Distribution itself
lives in tamias**: refs, transport, the install store, the fetch
cache, deploy, and service lifecycle are all `tamias` verbs. Peregrin
ships only the *provider* — `tamias-driver-peregrin` — which turns a
bundle's `provider:` block into an operator node config and lets
tamias render its service units.

The old `peregrin install/remove/list/info/update/pin/verify/cache/
bundle` verbs are **gone** — run them via `tamias`. The hidden
`peregrin provision {install,deploy,uninstall}` commands remain as the
provider entrypoints tamias shells out to.

## Ref grammar

```
tamias install ./docs-deploy                                  # filepath — dev loop
tamias install git:9aia/peregrin/libraries/docs-deploy        # shorthand (github), default branch
tamias install git:github.com/9aia/peregrin//libraries/docs-deploy#v0.2.0
tamias install git:git@github.com:9aia/peregrin//libraries/docs-deploy#main
tamias install git:https://gitlab.com/grp/sub/repo//dir   # any git url
tamias install https://9aia.com/bundles/docs-deploy-0.1.tgz   # non-git tarball
```

- **`git:<url>[//<subdir>][#<ref>]`** — `//` (go-getter convention)
  splits the repo path from the in-repo path; it's *required* for
  nested groups because bare `a/b/c/d` is ambiguous. `#ref` is a
  branch, tag or sha — the git ref *is* the version pin; the resolved
  commit sha is recorded regardless.
- **Without `//`**, a two-segment prefix is the repo and the remainder
  is the in-repo path — so `git:9aia/peregrin/libraries/docs-deploy` clones
  `github.com/9aia/peregrin` and takes `libraries/docs-deploy` from it.
  This rule only applies to *shorthand* paths: anything containing
  `://`, scp-style `host:path` (`git@…:`), or an explicit local path
  (`/x`, `~/x`, `./x`) parses whole.
- **Shorthand** — `org/repo` defaults to `github.com`;
  `host/org/repo` becomes `https://host/org/repo`. `git@…`,
  `ssh://`, `file://` and local paths pass through.
- **`./path`, `/path`, `~/path`, `nested/path`** → `source-fs`. A
  directory copies its payload (`.git/` excluded); a `.tgz`/`.tar.gz`/
  `.tar` file unpacks.
- **`http(s)://…​.tgz|.tar.gz`** → `source-http`, with an optional
  `#sha256:<hex>` (or `#sha256=<hex>`) digest pin verified on fetch.
- **`cf:`** refs hit the Cloudflare registry source.

## Store + fetch cache (tamias)

- **Install store** — `<name>@<version-or-sha>/` under
  `~/.local/share/tamias/` holds unpacked packages; each entry records
  the source ref, resolved sha, and payload digest.
- **Fetch cache** — content-addressed `~/.cache/tamias/fetch/<key>/`
  (`git-<sha>` for git, `sha256-<hex>` for content digests). Install
  is two-phase fetch→unpack: a warm cache makes repeat installs and
  air-gapped nodes free — a ref whose pin is already known skips the
  transport entirely on a cache hit.
- `tamias remove <name>` drops only the store entry; the cache evicts
  by age or `tamias cache clean --all`, never by `remove`.
- `tamias verify <name>` recomputes the payload digest and fails on
  mismatch; `tamias doctor` runs store/cache/registry/content-type
  diagnostics.

## `include:` composition

A manifest may declare `include: […]` — base bundles whose payloads
deep-merge *under* the local one. Entries are installed store names
(`name` or `name@version`) or source refs (installed on demand).
Semantics come from the ported loader: local wins, dicts merge
per-key, arrays concatenate local-first, and node `id` collisions
dedupe keeping the local definition — so an overlay that redefines a
node can't double-fire it. Earlier `include:` entries outrank later
ones. Conflicting `.yaml`/`.yml`/`.json` files merge document-wise;
every other conflict resolves whole-file.

## Tamias verbs (the ones that replaced `peregrin …`)

```
tamias install <ref>          # fetch → store (+ alias); dispatches the
                              # content-type driver's install verb
tamias add|update <name>      # update re-resolves the recorded source ref
tamias remove <name>          # drop store entry; cache untouched
tamias list                   # installed packages, refs, digests
tamias info <name>            # manifest, source ref, resolved sha, size
tamias pin <name> <ref>       # retarget the recorded ref
tamias verify <name>          # digest-check store contents vs recorded sha
tamias cache clean [--all]    # evict fetch cache
tamias deploy <name>          # render provider.services[] units + run the
                              # kind driver's deploy verb
tamias service enable|disable|status|logs   # unit lifecycle
tamias doctor                 # host diagnostics
tamias bundle pack <dir>      # authoring — dir + manifest → .tgz
```

## The `peregrin-bundle` provider

A bundle opts into the declarative runtime by declaring
`type: peregrin-bundle` (routed to `tamias-driver-peregrin`) plus a
`provider:` block in `tamias.yaml`:

```yaml
type: peregrin-bundle
provider:
  name: peregrin                  # SHARED identity — see below
  services:
    - name: campaign              # rendered as <provider>-<service>
      exec: >-
        ~/.local/bin/peregrin campaign serve ~/.config/peregrin/serve
        --listen 127.0.0.1:29173 --path /hook
        --history-dir ~/.local/share/peregrin/serve-history
        --queue-store-dir ~/.local/share/peregrin/serve-queues
      env_file: "~/.config/peregrin/env"
      workdir: "~"
      restart: on-failure
  ingress:
    - public_url: https://ci.example.com
      listen: 127.0.0.1:29173
      path: /hook
      hooks:
        - forge: github
          repo: 9aia/peregrin
          events: [push]
  verbs: [run, logs]
```

- **Shared provider identity (T8-7).** Every `peregrin-bundle`
  declares `provider.name: peregrin`, so all their `campaign` services
  render **one** unit — `peregrin-campaign.service` — whose
  `ExecStart` is `campaign serve ~/.config/peregrin/serve`, a
  directory the deploy driver fills with one generated node config per
  installed bundle. `campaign serve` hosts them all on ONE webhook
  listener (demux by `repository.full_name`). Shared-unit services
  MUST keep `exec` identical across bundles — last deploy wins the
  unit file.
- `tamias install <ref>` fetches + stores the package and dispatches
  the driver's `install` verb — a **no-op** for `peregrin-bundle` (the
  node config is generated by `deploy`, which sees the stable store
  dir; install only sees the evictable fetch cache).
- `tamias deploy <name>` dispatches the driver's `deploy` verb —
  `tamias-driver-peregrin` regenerates `~/.config/peregrin/<name>.yaml`
  as the operator node config: `include:` of the bundle's `main.yaml`,
  a `github-webhook` ingress node built from the manifest's
  `provider.ingress[].hooks` (`forge: github` only; `secret_env` =
  `PEREGRIN_WH_<OWNER>_<REPO>`), `config.public_url`, and
  `capabilities.secrets`. The same
  doc is dropped into the campaign-host dir
  `~/.config/peregrin/serve/<name>.yaml`. Tamias then renders
  `provider.services[]` as per-user units, written + daemon-reloaded.
- `tamias driver exec <pkg> <verb> [args…]` dispatches the driver's
  ops verbs (`hooks`, `health`, `status`, `logs`, `config-schema`,
  `capabilities`):
  - `hooks register|unregister [--dry-run] [--api-base <url>]
    [--token-env <NAME>]` — upsert/remove the forge webhooks
    `provider.ingress[].hooks[]` declare. Idempotent — the upsert
    matches on the exact hook URL (`public_url` + `path`). Secrets
    resolve by NAME through the env → keyring →
    `~/.config/peregrin/env` chain (`GITHUB_TOKEN` +
    `PEREGRIN_WH_<OWNER>_<REPO>`); values are never printed.
    **`--dry-run` first** — it prints the plan without touching the
    forge API or the secret chain. `public_url` fronts a tunnel the
    operator confirms, so `tamias deploy` never auto-registers.
  - `health` — JSON `{ok, probes[]}`: a github-webhook `ping`
    (`POST` to the ingress `listen`+`path`, `X-GitHub-Event: ping` →
    `200 pong`) per
    ingress that declares both, else `systemctl --user is-active` per
    provider unit.
  - `status` — JSON per-unit `is-active`/`is-enabled`.
  - `logs [--lines N]` — `journalctl --user` over the provider units
    (no follow — dispatch is bounded).
- `tamias uninstall <name>` removes the provider units + store entry;
  the driver's `uninstall` verb removes the node config, its
  campaign-host file, and any legacy `<name>.service` a pre-provider
  `deploy` wrote.
- `~` in `exec`/`env_file`/`workdir` expands at unit-render or
  `campaign serve` time — ExecStart argv is data, no shell.
- `peregrin provision {install,deploy,uninstall}` (with `--dir`,
  `--name`, optional `--config`) is the hidden provider entrypoint
  tamias invokes — it never writes service units itself.

`libraries/docs-deploy` is the pilot: `provider.name: peregrin` +
`services[].name: campaign` renders the shared
`peregrin-campaign.service` hosting every deployed bundle. Its
`tamias.yaml` ships `ingress[].hooks: []` — the bundle's `main.yaml`
is a full `peregrin import bupha` artifact that owns its ingress
inline (per-repo cmds/contexts/secret names), so manifest hooks would
only generate a duplicate webhook node. See the bundle's README for
the full operator flow.

## The `peregrin-node` type — the daemon itself

`libraries/peregrin-node` manages the node daemon, not a campaign:
`type: peregrin-node`, **no `main.yaml` payload**, and a
`provider.services[]` entry (`name: node`, `exec: peregrin serve`).
The driver recognizes the type and skips node-config generation on
`deploy` — there is no campaign to host — while `tamias deploy`
still renders the unit: `peregrin-node.service` (provider `peregrin`
+ service `node`, distinct from the shared `peregrin-campaign.service`).
The node reads `PEREGRIN_*` env, so `env_file` points at
`~/.config/peregrin/env`; `health`/`status` fall back to unit
`is-active` since there is no ingress probe.
