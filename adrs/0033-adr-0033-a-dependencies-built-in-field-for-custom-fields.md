+++
id = "ADR-0033"
title = "ADR-0033: A `dependencies` built-in field for custom fields"
status = "open"
priority = "medium"
+++

# ADR-0033: A `dependencies` built-in field for custom fields

## Status

Superseded by [handl ADR-0017](../../packages/handl/project/adrs/0017-dependencies-field.md). The dependencies field is now a language module system concern.

Proposed

## Context

ADR-0032 introduced user-defined workflow fields declared in `*.field.yml` files. Those fields are loaded on demand by the field resolver, but nothing in a workflow or library artifact explicitly says *which* custom fields it expects to use. This makes it hard for tooling and the engine to:

- Pre-load the right `*.field.yml` files before validation.
- Report a clear error when a workflow uses a custom field that is not installed or not in the search path.
- Understand which library nodes rely on which custom fields for documentation, `handl info`, and the exporter.

A built-in `dependencies` field lets authors and libraries declare the custom fields they need. The engine can then resolve all of those fields up front instead of discovering them one by one as it walks the workflow tree.

## Decision

Add a built-in `dependencies` field to the Peregrin workflow language. `dependencies` is a list of custom field names that the surrounding artifact requires to be resolvable before the workflow is validated or executed.

### Where `dependencies` is allowed

- On a `Workflow` at the top level.
- On a `Node`.
- In a `*.field.yml` file, when one custom field is built on top of another.
- In library artifacts such as `.node.yml` presets, so the library can say it uses a custom field.

### Syntax

`dependencies` is a list of dotted field names. Names may be bare (`delay`) for project-local fields or namespaced (`time.delay`, `resilience.timeout`) for library fields, following the convention from ADR-0032.

```yaml
# workflow.yaml
name: deploy
dependencies:
  - time.delay
  - resilience.timeout
  - notify

events:
  - type: build.done

nodes:
  - id: deploy
    driver: shell
    on: [build.done]
    params:
      cmd: ./deploy.sh
    dependencies:
      - resilience.timeout
```

```yaml
# libraries/fields/resilience/timeout.field.yml
name: resilience.timeout
dependencies:
  - time.sleep
schema:
  type: object
  required: [after]
  properties:
    after: { type: string }
    on_timeout: { type: string }
expand: |
  race:
    - {{ step }}
    - seq:
        - sleep: after
        - fail: on_timeout ?? "timed out"
```

### Resolution rules

1. The engine collects `dependencies` from all scopes in load order: workflow, then each `dependencies` block in `*.field.yml` files, then node `dependencies`.
2. It resolves each name with the same field resolver from ADR-0032: built-in fields first, then `*.field.yml` files discovered in the workflow directory, then `libraries/fields/`, then configured field paths.
3. If a `dependencies` entry cannot be resolved, validation fails with a list of the missing fields and the files that requested them.
4. A `dependencies` entry that points to a built-in core field is accepted and ignored because the engine already has it.
5. The list is de-duplicated across scopes; the same field is loaded once per workflow.

### Model changes

- Add an optional `dependencies: readonly string[]` to `Workflow`, `Node`, and the `FieldDefinition` model.
- The loader expands each custom field definition before it validates the workflow, so `dependencies` do not affect the runtime expansion algorithm once the fields are registered.
- `handl info <node>` and `handl info <field>` show the artifact's declared `dependencies`.
- `pereg lint` (ADR-0031) may warn when a `.node.yml` uses a custom field but does not declare it in `dependencies`.

### Library convention

Every library artifact that uses a custom field must declare it. This lets `handl validate` and `handl info` work correctly when the artifact is consumed in a workflow that does not otherwise load that field. For example:

```yaml
# libraries/git/worktree.node.yml
id: git.worktree
annotations:
  title: Check out a git worktree
  see: ADR-0033
dependencies:
  - shell
  - fs.exists
```

## Consequences

- Workflows and library artifacts become self-describing about the custom fields they need.
- The engine can preload all required `*.field.yml` files in a single discovery pass instead of failing mid-resolution.
- Validation errors are clearer because the engine can say which artifact requested a missing field.
- `handl info` and exporters can produce a complete dependency graph for a workflow or library node.
- Tooling can warn when a `dependencies` declaration is missing or stale.
- One more field must be added to the loader, validator, type generator, and `handl info` output.
- Library authors have another required convention to follow; `pereg lint` can help enforce it.

## Alternatives considered

- **Resolve custom fields lazily as they appear in the workflow.** Rejected. It makes error messages context-free, hides the relationship between a library and a custom field, and requires the user to guess which files to install.
- **Declare dependencies only at the workflow top level.** Rejected. Nodes and library artifacts are the places where custom fields are actually used; forcing all declarations to the top level would make reusable nodes fragile and harder to copy between workflows.
- **Use `include` for field files instead of a `dependencies` list.** Rejected. `include` is for structural inheritance and is path-based; `dependencies` is a logical name that the field resolver can locate from any configured field path or installed package.
- **Make `dependencies` a map with optional source paths.** Rejected for the first version. A simple list of names keeps the resolver in control of search paths. If package installation (ADR-0008) needs source pinning, a future ADR can extend the syntax.
