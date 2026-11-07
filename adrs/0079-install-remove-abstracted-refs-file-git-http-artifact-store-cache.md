+++
id = "ADR-0079"
title = "install/remove — abstracted refs (file/git/http), artifact store + cache"
status = "done"
created = "2026-11-04"
priority = "medium"
+++

# install/remove — abstracted refs (file/git/http), artifact store + cache

Status: **proposed** · Created: 2026-11-04

## Context

Peregrin is becoming the orchestration/execution substrate (ADR-0080)
— campaigns and workflow bundles ship to nodes, not just run from a
checkout. That needs a distribution verb: `peregrin install <ref>`
fetches a bundle, `peregrin remove <name>` drops it. The ref grammar
must be abstracted — a bundle is bytes + manifest whether it lives on
disk, in a git repo, or behind HTTP:

```
peregrin install ./ci-gate                                  # filepath — dev loop
peregrin install git:9aia/peregrin/libraries/ci-gate        # shorthand (github), default branch
peregrin install git:github.com/9aia/peregrin//libraries/ci-gate#v0.2.0
peregrin install git:git@github.com:9aia/peregrin//libraries/ci-gate#main
peregrin install git:https://gitlab.com/grp/sub/repo//dir   # any git url
peregrin install https://9aia.com/bundles/ci-gate-0.2.tgz   # non-git tarball
peregrin remove ci-gate
```

Full verb surface:

```
peregrin install <ref>          # fetch → store (+ alias)
peregrin remove <name>          # drop store entry; cache untouched
peregrin list                   # installed bundles, refs, digests
peregrin info <name>            # manifest, source ref, resolved sha, size
peregrin update <name>|<--all>  # re-resolve the source ref (the upgrade path for moving refs)
peregrin pin <name> <ref>       # retarget the recorded ref (refetches when digest differs)
peregrin verify <name>          # digest-check store contents vs recorded sha
peregrin cache clean [--all]    # evict fetch cache (age policy or full)
peregrin bundle pack <dir>      # authoring — dir + manifest → installable bundle
```

Provision/teardown verbs — bupha parity (today's
today's `bupha destroy webhook|service|worktrees`):

```
peregrin create --template <bundle>   # materialize unit + config
peregrin deploy webhook               # register repo hooks
peregrin destroy webhook              # remove hooks (per repo or all watched)
peregrin destroy service              # disable + remove the node unit
peregrin destroy worktrees            # clean leftover run worktrees
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
  ports-and-adapters discipline): `source-fs`, `source-git` (any
  clone URL — https/ssh/shorthand `org/repo` defaulting to github),
  `source-http` (tarball URL, optional digest). Each resolves to a
  bundle: manifest (`peregrin-bundle.toml`/`.yaml`) + payload
  (workflows, drivers-as-yaml per ADR-0026, field.yml per ADR-0032).
- **Ref grammar** — `git:<url>[//<subdir>][#<ref>]`. `//subdir`
  (go-getter convention) splits repo path from in-repo path —
  required because nested groups make bare `a/b/c/d` ambiguous.
  `#ref` is branch/tag/sha — the git ref *is* the version pin; no
  separate `@version` channel. Without `//`, a two-segment prefix
  is the repo and the remainder parses as the in-repo path.
- **Local store** — `~/.local/share/peregrin/store/<name>@<version>/`
  unpacked bundles; `name` aliases the installed entry so campaigns
  and `peregrin run` resolve `bundle:<name>` refs.
- **Cache** — `~/.cache/peregrin/fetch/<sha256>/` content-addressed
  downloads; install is a two-phase fetch→unpack so repeat installs
  and air-gapped nodes cost nothing. `remove` drops the store entry;
  cache evicts by age only.
- **Pins** — `#<branch|tag|sha>` on install; the resolved commit
  sha is recorded in the store manifest regardless, so
  `remove`/`list` can report "installed: ci-gate 0.2.0 (sha…)".
  No auto-upgrade — re-installing a moving ref (branch) is the
  explicit upgrade path.
- **Verb surface** — install/remove/list/info/update/pin/verify/
  cache/bundle-pack per the block above; `update` re-resolves the
  recorded source ref (moving refs like branches upgrade; pinned
  tags/shas re-confirm).

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
