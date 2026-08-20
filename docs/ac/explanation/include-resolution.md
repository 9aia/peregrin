# Include and snippet resolution

Four mechanisms, two levels of granularity, two naming schemes (path vs name).

## `include:` — merge (dict-level, path-based)

Works in any mapping. The loader finds the `include` key, loads the file,
and deep-merges its content into the parent mapping. The `include` key is
removed after resolution.

```yaml
params:
  include: snippets/prod-env.yaml
  cmd: "deploy.sh"
```

**Merge rules:**

- Scalars: local wins over included.
- Dicts: deep-merged recursively (local keys override included).
- Lists: concatenated (local first, included appended).

**Where it works:** any mapping in the YAML — top-level, node definitions,
`params` blocks, event schemas, nested dicts.

**Why merge, not replace:** you want to include a base config and override
specific fields. Merge gives you that for free.

## `${include(path)}` — inject (value-level, path-based)

Works in any string value. The loader finds the expression, loads the file,
and replaces the expression with the file's content.

```yaml
params:
  env: ${include(snippets/prod-env.yaml)}
  cmd: "deploy --region=${include(snippets/region.yaml)}"
```

**Two modes:**

- **Whole-string match:** the entire string is the expression. The file
  content replaces it, preserving type (dict, list, scalar).
- **Partial match:** the expression is part of a larger string. The file
  content is stringified and interpolated.

**Why inject, not merge:** sometimes you want to place a file's content at
a specific position, not merge it into the parent.

## `snippet: name` — merge (dict-level, name-based)

Snippets are named fragments defined in a top-level `snippets:` block. The
loader finds a `snippet` key in any mapping, looks up the named snippet,
deep-merges it into the parent, and removes the `snippet` key.

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

Snippets can inherit other snippets. To load snippet definitions from a file,
put a `snippets:` block in the file and `include:` it.

**Why snippets, not just includes:** includes are path-based — rename a file
or move a directory and every reference breaks. Snippets are name-based
indirection.

## `${snippet(name)}` — inject (value-level, name-based)

Works in any string value. The loader finds the expression, looks up the
named snippet, and replaces the expression with its content.

```yaml
snippets:
  pr-triggers: [pr.opened, pr.updated]
  model: sonnet

nodes:
  - id: review
    on: ${snippet(pr-triggers)}
    params:
      cmd: "devin run --model ${snippet(model)}"
```

## How they interact

All four are resolved in five phases:

1. `include:` resolution — deep-merge content from included files.
2. `snippets:` block — extract and resolve named snippet definitions.
3. `snippet: name` resolution — deep-merge named snippets into mappings.
4. `${snippet(name)}` resolution — resolve expressions in string values.
5. `${include(path)}` resolution — resolve expressions in string values.

All are recursive. Circular references are detected and raise an error.

## Path resolution

Paths are relative to the including file's directory, not the working
directory. Absolute paths also work.

```
/main.yaml          → include: snippets/lint.yaml → /snippets/lint.yaml
/snippets/sub.yaml  → include: ../shared.yaml    → /shared.yaml
```

## When to use which

| Need                                          | Use                                              |
| --------------------------------------------- | ------------------------------------------------ |
| Include a whole node definition               | `include: snippets/lint.yaml`                    |
| Include config defaults into a `params` block | `include: snippets/prod-env.yaml`                |
| Include event schemas at top level            | `include: [events.yaml]`                         |
| Inject a dict as a specific value             | `env: ${include(snippets/env.yaml)}`             |
| Inject a scalar into a string                 | `cmd: "deploy --region=${include(region.yaml)}"` |
| Inject a list as a specific value             | `on: ${include(snippets/triggers.yaml)}`         |
| Reusable node template by name                | `snippet: devin`                                 |
| Reusable value fragment by name               | `on: ${snippet(pr-triggers)}`                    |
| Share snippets across workflows               | `include: [snippets.yaml]` then `snippet: name`  |
