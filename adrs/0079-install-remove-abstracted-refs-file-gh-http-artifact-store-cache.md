+++
id = "ADR-0079"
title = "install/remove — abstracted refs (file/gh/http), artifact store + cache"
status = "open"
created = "2026-11-04"
priority = "medium"
+++

# install/remove — abstracted refs (file/gh/http), artifact store + cache

Status: **proposed** · Created: 2026-11-04

## Context

Peregrin is becoming the orchestration/execution substrate (ADR-0080)
— campaigns and workflow bundles ship to nodes, not just run from a
checkout. That needs a distribution verb: `peregrin install <ref>`
fetches a bundle, `peregrin remove <name>` drops it. The ref grammar
must be abstracted — a bundle is bytes + manifest whether it lives on
disk, in a git repo, or behind HTTP:

```
peregrin install ./ci-gate               # filepath — dev loop
peregrin install gh:9aia/peregrin/libraries/ci-gate  # repo path
peregrin install https://9aia.com/bundles/ci-gate-0.2.tgz
peregrin remove ci-gate
```

Today there is no artifact model at all — bundles are repo files
(`libraries/` stdlib, ADR-0029) or hand-copied directories. Installed
artifacts also need a **local store + cache**: fetches must be
content-addressed and offline-reusable (a gate node can't re-pull a
bundle on every boot), and pin/upgrade semantics have to be decided
(floating `latest` vs digested refs).

## Decision

- **`peregrin install <ref>` / `peregrin remove <name>`** — a new
  `Loader`-adjacent port (`source`/`fetch` family, same
  ports-and-adapters discipline): `source-fs`, `source-gh` (git clone/
  archive API, `gh:` or `github:org/repo/path` grammar, `@ref` pin),
  `source-http` (tarball URL, optional digest). Each resolves to a
  bundle: manifest (`peregrin-bundle.toml`/`.yaml`) + payload
  (workflows, drivers-as-yaml per ADR-0026, field.yml per ADR-0032).
- **Local store** — `~/.local/share/peregrin/store/<name>@<version>/`
  unpacked bundles; `name` aliases the installed entry so campaigns
  and `peregrin run` resolve `bundle:<name>` refs.
- **Cache** — `~/.cache/peregrin/fetch/<sha256>/` content-addressed
  downloads; install is a two-phase fetch→unpack so repeat installs
  and air-gapped nodes cost nothing. `remove` drops the store entry;
  cache evicts by age only.
- **Pins** — `@<tag|sha|digest>` on install; unpinned refs record
  the resolved digest in the store manifest so `remove`/`list` can
  report "installed: ci-gate 0.2.0 (sha…)". No auto-upgrade.
- **`peregrin list`** (or `install --list`) — installed bundles,
  source refs, digests.

## Consequences

- The 9aia factory shape lands: `libraries/` becomes the upstream
  for bundles like `ci-gate` (ADR-0080), and `peregrin create
  --template ci-gate` is `install` + enable-campaign — a
  parameterized install, not bespoke setup per repo.
- Bundle *integrity* matters: digested refs + optional sig checks
  are the growth path (ssh-sign like sturn events).
- Out of scope: a registry — refs point at existing hosts (fs, GH,
  HTTP); a `9aia.io/bundles` index is a later ADR if discovery
  becomes the pain point.
