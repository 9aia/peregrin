# Compose a workflow from fragments

Three composition mechanisms — includes (path-based) and snippets (name-based).

## 1. `include:` — merge whole files (dict-level)

Deep-merges the file's content into the parent mapping. Local values
override included. Works in any mapping — nodes, `params` blocks, top-level.

### Inline node includes

```yaml
# snippets/lint.yaml
id: lint
driver: shell
on: [start]
emits: [lint.passed, lint.failed]
params:
  cmd: "ruff check ."
```

```yaml
# main.yaml
nodes:
  - include: snippets/lint.yaml
  - include: snippets/test.yaml
  - id: custom
    on: [start]
    emits: [custom.done]
```

### `params` block includes

```yaml
# snippets/prod-env.yaml
env:
  REGION: us-east-1
timeout: 300
```

```yaml
# main.yaml
nodes:
  - id: deploy
    driver: shell
    on: [start]
    emits: [deploy.done]
    params:
      include: snippets/prod-env.yaml
      cmd: "deploy.sh"
      timeout: 60
```

### Top-level includes

```yaml
include: [events.yaml, config.yaml]
nodes:
  - id: lint
    on: [start]
    emits: [lint.done]
```

### Multiple includes

```yaml
params:
  include: [env.yaml, secrets.yaml]
  cmd: "deploy.sh"
```

## 2. `${include(path)}` — inject file content (value-level)

Replaces the expression with the file's content. Can inject any type:
dict, list, scalar. Also works as string interpolation.

### Inject a dict

```yaml
params:
  env: ${include(snippets/prod-env.yaml)}
  cmd: "deploy.sh"
```

### Inject a scalar with string interpolation

```yaml
# snippets/region.yaml
us-east-1

# main.yaml
params:
  cmd: "deploy --region=${include(snippets/region.yaml)} --force"
```

### Inject into any value position

```yaml
# snippets/triggers.yaml
- start
- manual.trigger

# main.yaml
nodes:
  - id: handler
    on: ${include(snippets/triggers.yaml)}
    emits: [handler.done]
```

## 3. `snippet: name` — merge named snippets (dict-level)

Snippets are named fragments defined in a top-level `snippets:` block.
Use `snippet: name` to merge one into any dict.

```yaml
snippets:
  devin:
    driver: shell
    emits: [devin.done, devin.failed]
    params:
      cmd: "devin run --prompt '${event.payload.prompt}'"
      timeout: 1800

nodes:
  - id: review
    snippet: devin
    on: [pr.opened]
    params:
      timeout: 3600
```

### Snippet inheritance

Snippets can inherit from other snippets via `snippet:` within a snippet
definition:

```yaml
snippets:
  base-llm:
    driver: shell
    emits: [llm.done, llm.failed]
    params:
      timeout: 600
  devin:
    snippet: base-llm
    params:
      cmd: "devin run"
      timeout: 1800
```

### Snippets from files

Put a `snippets:` block in a file and `include:` it:

```yaml
# snippets/devin-snippets.yaml
snippets:
  devin:
    driver: shell
    emits: [devin.done]
    params:
      cmd: "devin run"

# main.yaml
include: [snippets/devin-snippets.yaml]
nodes:
  - id: review
    snippet: devin
    on: [start]
```

## 4. `${snippet(name)}` — inject named snippet content (value-level)

Inject a snippet's content at any value position. Works as string
interpolation.

```yaml
snippets:
  pr-triggers: [pr.opened, pr.updated]
  prod-env:
    REGION: us-east-1
  model: sonnet

nodes:
  - id: review
    driver: shell
    on: ${snippet(pr-triggers)}
    emits: [review.done]
    params:
      env: ${snippet(prod-env)}
      cmd: "devin run --model ${snippet(model)}"
```

## Merge semantics (for `include:` and `snippet:`)

| Type          | Rule                                          |
| ------------- | --------------------------------------------- |
| Scalars       | Local wins over included                      |
| Dicts         | Deep-merged recursively (local overrides)     |
| Lists         | Concatenated (local first, included appended) |
| `include` key | Removed after resolution                      |
| `snippet` key | Removed after resolution                      |

## Shared rules

- Include paths are relative to the including file's directory.
- Snippets are resolved by name from the `snippets:` block (which may itself
  be populated via `include:`).
- Both includes and snippets are recursive.
- Circular references are detected and raise an error.

## File router — colocated discovery by suffix

Auto-discover workflow components by walking the file system. Files are
routed by **suffix pattern** — `<name>.<type>.<ext>`.

The router walks `.handl/` recursively first (precedence), then the
workflow's directory recursively. `.handl/` wins by name.

| Suffix                             | Kind         | What it does                             |
| ---------------------------------- | ------------ | ---------------------------------------- |
| `*.node.yml` / `*.node.yaml`       | Node         | Auto-included as node definitions        |
| `*.snippet.yml` / `*.snippet.yaml` | Snippet      | Auto-registered as snippets              |
| `*.event.json`                     | Event schema | Payload schema per event type            |
| `*.handler.py`                     | Handler      | Auto-imported as Python handler modules  |
| `*.config.yml` / `*.config.yaml`   | Config       | Deep-merged into workflow config         |
| `*.secret.yml` / `*.secret.yaml`   | Secret       | Auto-loaded as secrets                   |
| `.env`                             | Env          | KEY=VALUE file, loaded as secrets/config |

### Colocated layout

```
my-workflow.yaml
.handl/
  review/
    review.node.yml
    review.handler.py
    review.sh
    prompt.md
    pr.opened.event.json
  deploy/
    deploy.node.yml
    deploy.snippet.yml
    prod.config.yml
    tokens.secret.yml
    .env
```

```yaml
# main.yaml
name: my-workflow
nodes:
  - id: review
    snippet: devin
    on: [lint.done]
  - id: analyze
    driver: python
    handler: auto:review
    on: [devin.done]
```

### Payload schemas (`*.event.json`)

JSON Schema files that validate event payloads at runtime. The filename
determines which event type the schema applies to:

```json
{
  "type": "object",
  "properties": {
    "url": { "type": "string" },
    "author": { "type": "string" }
  },
  "required": ["url", "author"]
}
```

On validation failure:

- Default mode: logs a warning, emits `event.invalid`, and the invalid
  event still publishes.
- `--strict` mode: also emits `node.failed` and drops the invalid event.

### Placeholders in typegen

`handl typegen` produces schemas that accept `${...}` placeholders via `anyOf`
so editors do not flag them as type errors.

## See also

- [Write a workflow](../tutorials/write-a-workflow.md) — node fields and `params` blocks
