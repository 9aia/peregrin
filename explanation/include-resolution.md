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
not the working directory. `include` cycles error by path.

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
