# Include and snippet resolution

Composition happens at two levels: **file-level** (the YAML loader
expands `include:`/`snippet:` before the typed model ever sees the
document) and **distribution-level** (an installed bundle's manifest
`include:` deep-merges base bundles under the local payload). Both
use the same merge rules.

The normative semantics — merge order, discovery, the normalization
pass sequence — are specified in
[Campaign schema](../reference/campaign-schema.md) and
[Distribution](../reference/distribution.md); this page is the map
of *which mechanism solves which problem*.

## File level — four mechanisms, two axes

Two granularities (merge a mapping vs inject a value) × two naming
schemes (path vs name).

### `include:` — merge (dict-level, path-based)

Works in any mapping. The loader (`peregrin-loader-yaml`) loads the
file and deep-merges its content *under* the local keys; the
`include` key is removed after resolution.

```yaml
params:
  include: snippets/prod-env.yaml
  cmd: "deploy.sh"
```

Merge rules: local scalars win; dicts merge recursively per-key;
arrays concatenate local-first; node `id` collisions dedupe keeping
the local definition — an overlay that redefines a node can't
double-fire it.

**Why merge, not replace:** you want to include a base config and
override specific fields. Merge gives you that for free.

**`tamias:` refs** — the same `include:` key (and `${include(…)}`)
also accepts `tamias:<spec>//<path-inside-bundle>`, which resolves
`<path>` inside a tamias bundle's store entry
(`~/.local/share/tamias/store/<name>@<label>/`, the
`$TAMIAS_HOME`/`XDG_DATA_HOME`/`HOME` chain). `<spec>` resolves
**store-first**: the store-label spellings (`name`, `name@<label>`,
`@scope/name`) match installed entries — `@<label>` pins the store
label (`name@0.2.0` or the sha short form — what `tamias list`
reports, *not* the install-time source ref), and a bare `name` with
several installs picks the most recently installed entry, so pin for
determinism.

On a store **miss**, a ref-shaped spec is *fetched*: it is handed
verbatim to `tamias-install`'s pipeline — the same ref grammar
`tamias install` accepts — which resolves the ref, fetches through
the content-addressed fetch cache (`$TAMIAS_CACHE`/`XDG_CACHE_HOME`/
`HOME`), runs the manifest/signature/driver gates, and commits the
store entry with full provenance before the include resolves inside
it. This is a **network-capable load**: includes may fetch, and any
fetch failure is a hard error — never a silent fallback. Bundled
`drivers/` still need recorded consent (or `TAMIAS_ALLOW_DRIVERS=1`)
and declared `bins[]` are not deployed.

Fetchable specs are exactly the install refs:

- `git:org/repo#rev` (the `#rev` pin; the repo's in-repo subdir uses
  the two-segment form `git:org/repo/sub#rev` — a `git:` ref's own
  `//subdir` can't ride this spelling since `//` separates spec from
  in-bundle path)
- `cf:<account>/<bucket…>/<name>@<version>`
- `https://…/pkg.tgz#sha256:<hex>` (the `scheme://` is not mistaken
  for the separator)
- `<registry>:<name>@<version>` — a **named registry**: `<registry>`
  resolves against tamias's effective config (`$TAMIAS_CONFIG` or
  `./tamias.*` over `~/.config/tamias/tamias.*`), entry `id` before
  map key — the same config `tamias install` reads
- a local path (`/abs`, `./rel`, `~/…`)

Store-name shapes never fetch — tamias's ref grammar has no
bare-name or "latest" resolution — so `name`, `name@<label>`, and
`@scope/name` misses error with a `run 'tamias install <ref>' first`
hint. The in-bundle path may not escape the bundle root, and cycle
detection keys on the resolved store path like any other include.
See [Distribution](../reference/distribution.md) and tamias's
[Ref grammar](https://github.com/9aia/tamias/blob/main/docs/reference/refs.md).

```yaml
nodes:
  - id: gate
    on: [pr.opened]
    include: tamias:handl-gates//gates/rust.yaml
```

### `${include(path)}` — inject (value-level, path-based)

Works in any string value. Whole-string match → the file's parsed
content replaces it, preserving type (dict, list, scalar). Partial
match → stringified interpolation.

```yaml
params:
  env: ${include(snippets/prod-env.yaml)}
  cmd: "deploy --region=${include(snippets/region.yaml)}"
```

**Why inject, not merge:** sometimes you want a file's content at a
specific position, not merged into the parent.

### `snippet: name` — merge (dict-level, name-based)

Snippets are named fragments in a top-level `snippets:` block —
a **load-time-only** key that never reaches the typed `Campaign`.
`snippet: <name>` in any mapping deep-merges the body under local
keys; `snippet: <parent>` *inside* the snippets table is inheritance,
resolved to a fixpoint (local wins, cycles error).

```yaml
snippets:
  devin:
    driver: shell
    params:
      cmd: "devin run"
      timeout: 1800

nodes:
  - id: review
    snippet: devin
    on: [pr.opened]
    params:
      timeout: 3600
```

**Why snippets, not just includes:** includes are path-based —
rename a file and every reference breaks. Snippets are name-based
indirection inside a document.

### `${snippet(name)}` — inject (value-level, name-based)

```yaml
snippets:
  pr-triggers: [pr.opened, pr.updated]

nodes:
  - id: review
    on: ${snippet(pr-triggers)}
```

## How they interact

Resolution is recursive, ordered, and cycle-checked:

1. `include:` resolution (deep-merge included files)
2. `snippets:` block extraction + `snippet:`-in-table inheritance
3. `snippet: name` resolution in mappings
4. `${snippet(name)}` / `${include(path)}` string interpolation
5. Deep-merge pass (dicts per-key local-wins, arrays local-first)

Then normalization continues — `with:` rejection, poll
normalization, self-trigger `max_fires` defaults — per
[Campaign schema § Load-time normalization](../reference/campaign-schema.md).

**Path resolution:** relative to the *including* file's directory,
not the working directory — except `tamias:` refs, which resolve
against the absolute store root (so they work unchanged inside a git
worktree). `include` cycles error by resolved path.

## Distribution level — bundle `include:`

A bundle manifest (`tamias.yaml`) may declare its own
`include: […]` — installed store names (`name` or `name@version`)
or source refs installed on demand — whose payloads merge under the
local one with the *same* rules (local wins, earlier entries outrank
later). This is how a library bundle carries shared node fragments
without vendoring them. See
[Distribution § `include:` composition](../reference/distribution.md).

## When to use which

| Need | Use |
| --- | --- |
| Include a node definition or config defaults | `include: snippets/lint.yaml` |
| Inject a dict/list/scalar as a specific value | `${include(file)}` / `${snippet(name)}` |
| Reusable node template by name | `snippet: devin` |
| Reusable value fragment by name | `on: ${snippet(pr-triggers)}` |
| Share fragments across workflows in one repo | `include:` a `snippets:` file, then `snippet: name` |
| Share fragments across machines/git refs | a bundle — `tamias install`, manifest `include:` |
| Compose a workflow from a bundle's files | `include: tamias:<spec>//<path>` (store label, or an install ref fetched on miss) |
