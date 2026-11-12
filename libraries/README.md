# Peregrin standard library

The `libraries/` directory holds the reusable building blocks for Peregrin
workflows. Each file is a domain-namespaced artifact that the loader can
`include`, expand as a `snippet`, or run as a workflow.

## What is here

| Directory   | Id pattern   | Purpose                                            |
| ----------- | ------------ | -------------------------------------------------- |
| `agent/`    | `agent.*`    | **Deprecated (PL-015).** Agentic node presets kept only for `meta/workflows/create`. |
| `circuit-breaker/` | `circuit-breaker.*` | Failure-count trip, cooldown, half-open probe, dispatch gate. |
| `dead-letter/` | `dead-letter.*` | `*.dead`/`*.failed` capture, inventory, replay.  |
| `display/`  | `display.*`  | User-facing reports and abort controls.            |
| `lang/`     | `lang.*`     | handl language reusable field patterns.            |
| `map/`      | `map.*`      | Fan-out (planner + `runner` inject) and `fan_in` reduce — map/reduce as fragment (ADR-0021). |
| `meta/`     | `meta.*`     | Meta-operations and workspace introspection.       |
| `peregrin/` | `peregrin.*` | Peregrin-project-specific orchestrations.          |
| `retry/`    | `retry.*`    | Failure re-emission with an attempt budget and `.dead` terminal. |
| `test/`     | `test.*`     | Test execution and reporting.                      |

`peregrin/` is the only project-specific namespace. The rest are general
purpose and may be copied into other projects.

Retired (PL-015): the `common/` ready-made workflows (`new-feat`,
`refactor`) and the `git/`/`github/` snippet dirs were removed — the
self-contained replacements live in the workspace `workflows/` bundles
(`new-feat`, `refactor`, `pr-heal`, `docs-deploy`). `agent/` survives
only because `meta/workflows/create` still includes it.

## Installable bundles (ADR-0079/0080)

Directories carrying a `tamias.yaml` manifest are installable
artifacts (`tamias install <ref>` — distribution is tamias's job),
not loader fragments:

| Directory  | Bundle    | Purpose                                                              |
| ---------- | --------- | -------------------------------------------------------------------- |
| `ci-gate/` | `ci-gate` | Push → worktree-at-sha → shell run → `ci/<campaign>` status gate. **Migrating to `9aia/bupha//libraries/ci-gate` (`type: bupha`) — see its README; do not extend.** |
| `docs-deploy/` | `docs-deploy` | Push to `refs/heads/docs` → worktree → `bun install` + `docs:deploy` (vitepress → gh-pages) → `docs/deploy` status. |
| `release/` | `release` | Release pipeline driving **tarsi** `tarsi-serve` ops (`plan` → gate → `release`) on release-ref pushes — tarsi is the org release plane (project ADR-0005). |

Each documents its operator params in `params.schema.json` + its
`README.md` — see `peregrin import bupha` for the fleet-shaped
equivalent.

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
  - include: ../libraries/retry/retry.node.yml
    id: mytask.retry
    on: [mytask.failed]
    forward: [mytask.request]
    until: [mytask.done, mytask.dead]
    max_fires: 2
```

The `include` merges the library file with the overrides you provide. The
`id`, `on`, `emits`, and any `params` fields you set take precedence.

## Use a reusable prompt

Place a `.prompt.yml` file next to the workflow or node that uses it, then
reference it with `include`:

```yaml
# libraries/meta/workflows/create/main.yaml
nodes:
  - include: ../../../agent/generate.node.yml
    params:
      prompt: ${include(../../generate.prompt.yml)}
```

```yaml
# libraries/meta/generate.prompt.yml
|
  Generate an agent-campaign workflow YAML and save it to ${WORKFLOW_PATH}.
  Name the workflow ${WORKFLOW_NAME}.
```

`.prompt.yml` files are not auto-discovered. The explicit `include` keeps the
dependency visible.

## Add a new library node

1. Create `libraries/<domain>/<name>.node.yml`.
2. Use the dotted namespace `id: <domain>.<name>`.
3. Add an `annotations` block with at least `title`, `description`, and `tags`.
4. Set `since: "0.1.0"` for the initial release.
5. Set `timeout`, `emit` defaults, and `max_fires` for resilience.
6. Run `peregrin campaign validate` on a workflow that uses the node.

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

`retry/`, `circuit-breaker/`, and `dead-letter/` are resilience
*fragments* (ADR-0082 #6): subscriber-pattern workflows you `include:`
at top level — the merge dedupes `nodes:` by `id`, so re-declaring a
node replaces its wiring. Each directory is also a bundle
(`tamias.yaml`) and ships `.node.yml` presets for per-node
composition plus a `.field.yml` params surface where applicable. Full
contracts live in the `main.yaml` header comments; e2e coverage in
`crates/peregrin-node/tests/libraries.rs`.

Library nodes that run external commands or agents should set:

- `timeout` in `params`.
- `emit` mapping with `default` fallback.
- `max_fires` unless the node is intentionally one-shot.
- `retry` or `circuit_breaker` for fallible drivers.

Use `strict: true` in ready-made workflows when all event and payload
contracts are complete.

## Validation

Run `peregrin campaign validate <workflow>` on every workflow that uses a
library node. Add or update a node in `crates/peregrin-node/tests/libraries.rs`
if the node is widely used.

## References

- `project/adrs/0029-adr-0029-peregrin-standard-library-libraries.md` — design rationale and conventions.
- `project/adrs/0009-adr-0009-annotations-in-workflow-yaml.md` — the `annotations` field.
- `docs/reference/campaign-schema.md` + `docs/explanation/include-resolution.md` —
  `include`, `snippet`, and merge behavior as shipped.
