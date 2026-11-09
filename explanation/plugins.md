# Plugins — YAML composition, now with distribution

v1's thesis still holds: **YAML files ARE plugins.** A plugin is a
fragment someone includes — no code, no registration, no plugin API.
What changed is that v2 adds a *distribution* layer on top: bundles,
install refs, a store, and ops verbs (ADR-0079). The plugin unit is
still YAML; now it also ships.

## The plugin is still a file

```yaml
# plugins/notify/slack.yaml — a fragment
id: notify-slack
driver: shell
on: [notify.requested]
emits: [notify.done]
params:
  cmd: 'curl -X POST -H "Content-type: application/json" --data "{\"text\":\"${event.payload.message}\"}" ${secret.SLACK_WEBHOOK_URL}'
```

A campaign "installs" it with `include:` — see
[Include and snippet resolution](include-resolution.md). Install =
drop a file; use = `include:`; version = git; override = deep-merge;
remove = delete the include line.

## The bundle is the shippable plugin

When a plugin must cross machines — a CI gate, a release pipeline,
a shared fragment library — it becomes a *bundle*: a
`tamias.yaml` manifest plus a payload of workflows,
`.node.yml`, `.driver.yml`, `.field.yml` files.

```bash
peregrin install git:9aia/peregrin//libraries/ci-gate   # → the store
peregrin list && peregrin info ci-gate
peregrin create --template ci-gate --repo owner/repo  # materialize
```

- **Install** — fetch from fs/git/http, unpack into
  `~/.local/share/peregrin/store/<name>@<version>/`, record the
  resolved pin (commit sha or `sha256:` digest). Refs pin by git ref
  or content digest; the fetch cache makes warm installs free.
- **Compose** — a manifest's `include:` deep-merges base bundles
  under the local payload, same rules as file-level includes.
- **Instantiate** — `peregrin create --template <bundle>` writes the
  operator's node config (the bundle's `main.yaml` included,
  `config:` params merged, an ingress node per `--repo`) plus a
  service unit. The *operator* config is where secrets and watched
  repos bind — a bundle never ships `params.repos`/`secret_env`
  values.

The ref grammar, store layout, cache, and verbs are specified in
[Distribution](../reference/distribution.md).

## Why not a real plugin system?

A plugin system adds registration, versioning, dependency
resolution, loading order, namespace management. Bundles get
versioning and distribution from *git and tarballs* — the same
mechanisms the ecosystem already runs — while the plugin itself
stays a YAML document the loader already understands. The ceremony
stays out of the engine.

## See also

- [Distribution](../reference/distribution.md) — refs, store, cache,
  verbs
- [Include and snippet resolution](include-resolution.md) — the
  file-level half
- [Your first gate](../tutorials/first-gate.md) — the install →
  create → deploy flow end to end
