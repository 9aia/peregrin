# Distribution — bundle refs, store, cache, verbs

ADR-0079. A *bundle* is a manifest (`peregrin-bundle.yaml` / `.yml` /
`.toml`) plus a payload of workflows, `.node.yml`, `.driver.yml` and
`.field.yml` files. `peregrin install <ref>` fetches one from the
filesystem, a git remote, or an HTTP tarball, unpacks it into a local
store, and records the resolved pin.

## Ref grammar

```
peregrin install ./ci-gate                                  # filepath — dev loop
peregrin install git:9aia/peregrin/libraries/ci-gate        # shorthand (github), default branch
peregrin install git:github.com/9aia/peregrin//libraries/ci-gate#v0.2.0
peregrin install git:git@github.com:9aia/peregrin//libraries/ci-gate#main
peregrin install git:https://gitlab.com/grp/sub/repo//dir   # any git url
peregrin install https://9aia.com/bundles/ci-gate-0.2.tgz   # non-git tarball
```

- **`git:<url>[//<subdir>][#<ref>]`** — `//` (go-getter convention)
  splits the repo path from the in-repo path; it's *required* for
  nested groups because bare `a/b/c/d` is ambiguous. `#ref` is a
  branch, tag or sha — the git ref *is* the version pin; the resolved
  commit sha is recorded regardless.
- **Without `//`**, a two-segment prefix is the repo and the remainder
  is the in-repo path — so `git:9aia/peregrin/libraries/ci-gate` clones
  `github.com/9aia/peregrin` and takes `libraries/ci-gate` from it.
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

## Store + fetch cache

- **Store** — `~/.local/share/peregrin/store/<name>@<version-or-sha>/`
  holds unpacked bundles. `name` aliases the entry so campaigns and
  `peregrin run` can resolve `bundle:<name>` refs. Each entry carries
  `.peregrin-store.json`: the source ref, resolved sha (commit sha or
  `sha256:` digest), payload digest, and installed-at.
- **Fetch cache** — `~/.cache/peregrin/fetch/<key>/` is
  content-addressed (`git-<sha>` for git, `sha256-<hex>` for content
  digests). Install is two-phase fetch→unpack: a warm cache makes
  repeat installs and air-gapped nodes free — a ref whose pin is
  already known (a `#sha` or a `#sha256:` tarball digest) skips the
  transport entirely on a cache hit.
- `remove` drops only the store entry; the cache evicts by age or
  `cache clean --all`, never by `remove`.

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

## Verbs

```
peregrin install <ref>          # fetch → store (+ alias)
peregrin remove <name>          # drop store entry; cache untouched
peregrin list                   # installed bundles, refs, digests
peregrin info <name>            # manifest, source ref, resolved sha, size
peregrin update <name>|--all    # re-resolve the recorded source ref
peregrin pin <name> <ref>       # retarget the recorded ref
peregrin verify <name>          # digest-check store contents vs recorded sha
peregrin cache clean [--all]    # evict fetch cache (--days N, default 30)
peregrin bundle pack <dir>      # authoring — dir + manifest → .tgz
```

- `update` re-resolves the recorded source ref — the upgrade path for
  moving refs like branches; pinned tags/shas re-confirm.
- `pin` retargets the recorded ref and refetches when the resolved
  digest differs.
- `verify` recomputes the payload digest and fails on mismatch.
- `bundle pack` writes `<name>-<version>.tgz` (or `--out`), installable
  via `peregrin install ./x.tgz` or an `https://` URL.
- Names accept `name` or `name@version`; a bare name with several
  installed versions is an ambiguity error.
