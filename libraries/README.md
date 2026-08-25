# Peregrin standard library

The `libraries/` directory holds the reusable building blocks for Peregrin
workflows. Each file is a domain-namespaced artifact that the loader can
`include`, expand as a `snippet`, or run as a workflow.

## What is here

| Directory   | Id pattern   | Purpose                                            |
| ----------- | ------------ | -------------------------------------------------- |
| `agent/`    | `agent.*`    | Agentic node presets: base, fix, review, generate. |
| `common/`   | unprefixed   | Cross-cutting, ready-made workflows.               |
| `display/`  | `display.*`  | User-facing reports and abort controls.            |
| `git/`      | `git.*`      | Git worktrees, commits, and branches.              |
| `github/`   | `github.*`   | GitHub PRs, reviews, and merges.                   |
| `lang/`     | `lang.*`     | handl language reusable field patterns.            |
| `meta/`     | `meta.*`     | Meta-operations and workspace introspection.       |
| `peregrin/` | `peregrin.*` | Peregrin-project-specific orchestrations.          |
| `test/`     | `test.*`     | Test execution and reporting.                      |

`peregrin/` is the only project-specific namespace. The rest are general
purpose and may be copied into other projects.

## File conventions

- `.node.yml` — a reusable, partial node meant to be `include`d and
  customized. It should have an `id`, `driver`, default `params`, and an
  `annotations` block.
- `.snippet.yml` — a named snippet for scalar or structural reuse via
  `snippet: <name>` or `${snippet(name)}`.
- `.prompt.yml` — domain-local prompt text, included with
  `params.prompt: ${include(...)}`.
- `.event.json` — a reusable payload schema for an event type.
- `.field.yml` — a reusable handl language field pattern (see `lang/`).
- `.yaml` with a top-level `name:` — a runnable workflow.

## Use a library node in your workflow

```yaml
# my-workflow/main.yaml
name: my-workflow

nodes:
  - include: ../libraries/git/worktree.node.yml
    id: my.setup
    on: [start]
    params:
      repo: "${config.repo}"
      worktree_base: "${config.worktree_base}"
```

The `include` merges the library file with the overrides you provide. The
`id`, `on`, `emits`, and any `params` fields you set take precedence.

## Use a reusable prompt

Place a `.prompt.yml` file next to the workflow or node that uses it, then
reference it with `include`:

```yaml
# libraries/common/new-feat/main.yaml
nodes:
  - include: ../../agent/base.node.yml
    id: new-feat.adr
    on: [worktree.ready]
    params:
      permission_mode: normal
      timeout: 600
      prompt: |
        ${include(./new-feat.prompt.yml)}
        Write or update the ADR.
```

```yaml
# libraries/common/new-feat/new-feat.prompt.yml
|
  We are implementing feature "${state.feat_id}": ${state.feat_description}.
  Working in ${state.worktree_path}.
```

`.prompt.yml` files are not auto-discovered. The explicit `include` keeps the
dependency visible.

## Add a new library node

1. Create `libraries/<domain>/<name>.node.yml`.
2. Use the dotted namespace `id: <domain>.<name>`.
3. Add an `annotations` block with at least `title`, `description`, and `tags`.
4. Set `since: "0.1.0"` for the initial release.
5. Set `timeout`, `emit` defaults, and `max_fires` for resilience.
6. Run `handl validate` on a workflow that uses the node.

## Annotations

Every artifact should declare an `annotations` block. Conventional keys:

- `title` — short display name.
- `description` — one-sentence explanation.
- `author` — person or team responsible.
- `tags` — list of labels for search and grouping.
- `since` — version when introduced. Use `"0.1.0"` for the initial stdlib.
- `deprecated` — boolean or deprecation message.
- `example` — short example value or invocation.
- `see` — link to related ADR or documentation.

The engine ignores annotations, so they have no runtime cost.

## Composition rules

1. Use `include` to inherit structure from another file or inject a prompt.
2. Use `snippet` to reuse small values or maps.
3. Use `params` to set local `params` for a node or driver.
4. Use `events` with `fields` or `schema` to define event payload contracts.

## Resilience

Library nodes that run external commands or agents should set:

- `timeout` in `params`.
- `emit` mapping with `default` fallback.
- `max_fires` unless the node is intentionally one-shot.
- `retry` or `circuit_breaker` for fallible drivers.

Use `strict: true` in ready-made workflows when all event and payload
contracts are complete.

## Validation

Run `handl validate <workflow>` on every workflow that uses a library node. Add
or update a node in `packages/tests` or `libraries/tests` if the node is
widely used.

## References

- `project/adrs/0029-stdlib-design.md` — design rationale and conventions.
- `project/adrs/0009-annotations.md` — the `annotations` field.
- `packages/handl-file-router/src/index.ts` — file discovery suffixes.
- `packages/handl-loader-yaml/src/loader.ts` — `include`, `snippet`, and merge
  behavior.
