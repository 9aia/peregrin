+++
id = "ADR-0029"
title = "ADR-0029: Peregrin standard library (libraries/)"
status = "open"
priority = "medium"
+++

# ADR-0029: Peregrin standard library (libraries/)

## Status

Superseded by [handl ADR-0013](../../packages/handl/project/adrs/0013-standard-library.md). The standard library is now a language platform concern. Peregrin-specific library artifacts (orchestration nodes, event schemas) remain under Peregrin's scope.

Proposed

## Context

The `libraries/` directory is the growing collection of reusable workflow
building blocks used by the Peregrin engine. Today it contains `.node.yml`
partials, `.snippet.yml` values, `.event.json` schemas, and ready-made
`.yaml` workflows. The existing files work, but they have grown ad-hoc:

- Namespaces are inconsistent. Some nodes use a `common.*` prefix while
  others do not, and event types are not namespaced to their domain.
- Reuse is mostly through `include` paths, but there is no documented pattern
  for when to use `include`, `snippet`, `params` overrides, or `.driver.yml`
  presets.
- Prompts are duplicated inline in multiple nodes instead of living as
  reusable, domain-local files.
- Documentation is in YAML comments. Comments are not machine-readable and
  drift from the runtime model. The `annotations` field (ADR-0009) is not
  used consistently.
- Resilience controls such as `max_fires`, `retry`, `circuit_breaker`,
  `timeout`, and `emit` defaults are present in some files and missing in
  others.
- There is no classification that tells a workflow author whether a file is
  a snippet, a node preset, an event schema, or a runnable workflow.
- The layout does not distinguish general-purpose building blocks from
  Peregrin-project-specific orchestrations.

A systematic design is needed so `libraries/` becomes a resilient,
composable, general-purpose standard library that other projects can adopt
and extend.

## Decision

`libraries/` is the Peregrin standard library. It is a collection of
reusable, domain-namespaced YAML artifacts consumed by `handl-loader-yaml`,
`handl-file-router`, `handl-engine`, and `handl validate`. It is not a TypeScript
package; it ships with the workspace and is versioned alongside the engine.

### File taxonomy

Every file in `libraries/` must declare its role by extension and by
`annotations.kind` when that role is not obvious from the extension:

| Extension | Role | Example |
| --- | --- | --- |
| `.node.yml` | Reusable node preset. May be `include`d and overridden. | `git/worktree.node.yml` |
| `.snippet.yml` | Named snippet for scalar or structural reuse. | `common/messages/success.snippet.yml` |
| `.prompt.yml` | Domain-local prompt text, included by `params.prompt: ${include(...)}`. | `common/new-feat/new-feat.prompt.yml` |
| `.event.json` | Reusable payload schema for shared event types. | `github/events/pr.opened.event.json` |
| `.yaml` with `name:` | Runnable workflow. | `common/new-feat/main.yaml` |

Prompts are `.prompt.yml` files placed in the domain folder that consumes
them. They are not auto-discovered; the explicit `include` keeps the
dependency visible and lets a prompt stay next to the workflow or node it
serves.

`.driver.yml` presets are a future extension from ADR-0026. When supported,
they take the same namespace rules and are placed next to the nodes they
serve.

### Namespace convention

- `libraries/<domain>/` is the top-level namespace.
- Artifact `id` and event `type` use dotted form: `<domain>.<name>` for
  every domain except `common`, which is the default namespace and is
  unprefixed.
- `peregrin` is reserved for project-specific Peregrin build and release
  orchestrations.
- New domains are added as `libraries/<domain>/`. No flat mixing of concerns.

Examples:

- `libraries/agent/base.node.yml` with `id: agent.base`
- `libraries/git/worktree.node.yml` with `id: git.worktree`
- `libraries/common/new-feat/main.yaml` with `name: new-feat`
- `libraries/peregrin/validate.node.yml` with `id: peregrin.validate`

### Composition patterns

Use the existing loader features in this priority order:

1. `include` for structural inheritance from another file.
2. `include(...)` for injecting prompt or message files as strings.
3. `snippet` or `${snippet(name)}` for atomic reuse of a value or map.
4. `params` for local parameterization of a node or driver.
5. `events` with `fields` and `schema` for event contracts.

Every `.node.yml` must document its required `params` overrides and expected
emits in `annotations.description` and `annotations.example`.

### Resilience conventions

Every `.node.yml` that runs a driver must include:

- `timeout` in `params`.
- `emit` mapping with a `default` fallback event.
- `max_fires` with an explicit value unless the node is intentionally
  one-shot.
- `retry` or `circuit_breaker` for external or fallible drivers.

Runnable workflows should set `strict: true` when the event contracts and
node `emits` are complete. This lets `handl validate` catch missing or
mismatched events before runtime.

### Documentation conventions

Every artifact has an `annotations` block with at least:

- `title` — short display name.
- `description` — one-sentence explanation.
- `author` — person or team responsible.
- `tags` — list of labels for grouping and search.
- `since` — version when introduced.
- `see` — link to related ADR or documentation.

Set `since` to `"0.1.0"` for the initial stdlib release. Bump the value
per artifact only on a breaking change or major revision.

YAML comments are reserved for explaining "why", not "what".

### Categories

The standard library is organized into these domains. New domains require a
new top-level directory and an ADR or README update.

| Domain | Purpose |
| --- | --- |
| `core` | Engine primitives: shell, http, timer, decision, expression. |
| `system` | Process controls: human, display, abort, report, safety. |
| `git` | Git operations: worktree, branch, commit, tag. |
| `github` | GitHub operations: PR, review, merge, issue. |
| `agent` | Agentic node presets: base agent, fix, review, generate. |
| `test` | Test execution and result reporting. |
| `common` | Cross-domain ready-made workflows. |
| `peregrin` | Peregrin project-specific orchestrations. |

### Validation

- `handl validate <workflow>` is run for every `.yaml` workflow in CI.
- `handl info <node>` shows the `annotations` and contract of a library node.
- Library nodes are individually exercised in the `packages/tests` suite or in
  a new `libraries/tests` directory.

## Consequences

- The `libraries/` surface becomes predictable. Authors can find, compose,
  and override nodes without reading every file.
- Namespaced ids and event types reduce the risk of collisions when multiple
  libraries are loaded into one workflow.
- Prompts live next to the workflows they serve, so a domain is
  self-contained and easy to copy or version.
- `annotations` give tooling such as `handl info`, the exporter, and future
  library browsers structured metadata.
- Resilience defaults make library nodes safer to include in production
  workflows.
- Existing files must be renamed and annotated to follow the convention. This
  is a breaking change for workflows that use current ids or include paths.

## Alternatives considered

- **Keep `libraries/` as an ungoverned directory of examples.** Rejected. The
  files are already relied upon by production workflows, so they need
  versioning, validation, and clear contracts.
- **Make `libraries/` a published package under `packages/`.** Rejected. YAML
  assets are consumed by the engine at runtime; packaging them as a
  TypeScript package adds a build and release step without benefit.
- **Use a separate metadata file (`.meta.yml`) for each artifact.** Rejected.
  Annotations live inside the artifact per ADR-0009, which keeps metadata
  from drifting.
- **Use a top-level `prompts/` technical layer.** Rejected. Grouping prompts
  by function separates them from the workflows they serve. Domain folders
  keep prompts, nodes, and workflows together.
