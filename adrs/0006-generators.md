# ADR-0006: Campaign generators

## Status

Proposed

## Context

Peregrin workflows are declarative YAML files. A workflow is a contract that
describes events, nodes, drivers, and their relationships. Because this
structure is explicit and versionable, it can be used to derive other artifacts
automatically and deterministically. The same YAML that drives a campaign can
also drive documentation, visualisations, tests, and stubs for language ports.

Today the project does not provide any tooling to turn a workflow file into
other representations. This means that:

- Authors maintain READMEs and diagrams by hand. These drift out of sync with
  the workflow.
- New team members cannot see the shape of a campaign without reading YAML.
- Cross-language ports (Python, Go) lack a way to generate consistent stubs from
  the canonical definition.
- Event types, payload schemas, and driver contracts are documented separately
  from the source of truth.

A generator layer turns the workflow file into a reusable artefact factory. The
output is deterministic because the input is declarative.

## Decision

Peregrin will provide a `Generator` abstraction and a `peregrin generate`
command. The generator takes a `Workflow` (loaded through `ac-loader-yaml`) and
emits one of several target formats.

### Generator targets (proposed)

1. **Mermaid / Graphviz diagram**
   Render the bipartite event graph from `events`, `on`, and `emits`. Distinct
   shapes for events and nodes. Optional highlighting of poll, retry, and
   `runner` nodes.

2. **Markdown overview**
   A runbook per workflow: event glossary, node table, driver callouts,
   `with` parameters, `emits` per node, and a prose description of the campaign
   shape.

3. **Event schema stubs**
   TypeScript, Python, or Go types from `events` and `payload_schemas`.

4. **Test skeletons**
   Per-node stub tests for each emitted event and driver invocation.

5. **Prompt templates**
   Rendered `with.prompt` values for `agent` and `human` nodes, pre-filled with
   placeholder hints.

6. **Campaign map for libraries**
   Aggregated overview of all workflows under an `include` tree.

### Command surface (proposed)

```bash
peregrin generate <workflow> --format mermaid
peregrin generate <workflow> --format markdown --out docs/campaigns/
peregrin generate <workflow> --format python-stubs
peregrin generate <dir> --format campaign-map
```

The generator must not execute the workflow. It only reads and renders the
static definition. Runtime-only concerns such as conditions, retries, and
dynamic node injection are documented as "resolved at runtime".

## Consequences

- Workflow authors get diagrams and docs that stay in sync with the YAML.
- The DSL becomes the single source of truth for campaign structure, not just
  execution.
- Cross-language ports can generate starter types and stubs from the same
  canonical model.
- We must keep the `Workflow` model stable enough that generators do not break
  between releases. Changes to `ac-core` may require generator updates.
- Generated artifacts are static. They describe the _possible_ event graph, not
  the _actual_ runtime path.

## Alternatives considered

- **Hand-maintain diagrams and docs.** Rejected because drift is inevitable.
- **Generate artifacts inside `apps/website` only.** Rejected because other
  consumers (CLI, IDE extensions, ports) also need access to the same output.
- **Make generators a driver or meta-workflow.** Rejected for now. Generators
  are build-time/read-time tooling; running them as campaign side effects would
  complicate the runtime model without clear benefit.
