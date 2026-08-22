# ADR-0009: Annotations in workflow YAML

## Status

Archived

## Context

Peregrin workflows are written in YAML and consumed by both the engine and
tooling such as the exporter (ADR-0006) and the CLI. Today the YAML only carries
runtime-relevant fields: events, nodes, drivers, and `params` values. There is no
place for human-readable metadata: descriptions, examples, deprecation notices,
authors, tags, or documentation links. This forces the exporter, documentation
generators, and library browsers to infer meaning from identifiers or to
duplicate metadata outside the workflow file.

When `ac export` produces a Markdown runbook or a Mermaid diagram, it should be
able to use annotations to generate useful prose without requiring a separate
metadata file.

## Decision

Peregrin will support an optional `annotations` field on `Workflow`, `Node`, and
`EventSchema`. The value is a plain `Record<string, unknown>` that the engine
passes through without interpretation. A small set of conventional keys will be
recognised by documentation and exporter tooling.

### Example

```yaml
workflow: deploy

annotations:
  title: Deploy pipeline
  description: Run tests, ask a human, generate release notes, and deploy.
  author: team-platform
  tags: [deploy, release, human-in-the-loop]
  since: "1.2.0"
  deprecated: false
  see: https://docs.example.com/patterns/deploy

events:
  - type: pr.opened
    annotations:
      description: A pull request was opened.
      payload_schema: pr.schema

nodes:
  - id: tests
    driver: shell
    on: [pr.opened]
    emits: [build.done, build.failed, build.timeout]
    annotations:
      description: Run the test suite.
      example: npm test
    params:
      cmd: "npm test"
```

### Conventional keys

These keys are reserved for tooling; other keys are allowed but ignored by the
engine and default exporters:

- `title` — short display name.
- `description` — one-sentence explanation.
- `author` — person or team responsible.
- `tags` — list of labels for grouping and search.
- `since` — version when the workflow or node was introduced.
- `deprecated` — boolean or deprecation message.
- `example` — short example value or invocation.
- `see` — URL to longer documentation.

### Model changes

- Add `annotations: Readonly<Record<string, unknown>>` to `Workflow`, `Node`,
  and `EventSchema` constructors and `fromDict`/`toDict`.
- The default for `annotations` is `{}`.
- `toDict` omits `annotations` when it is empty.

## Consequences

- The exporter (ADR-0006) can generate richer Markdown runbooks and diagrams
  using `title`, `description`, and `example`.
- The `ac info` CLI command can show workflow and node metadata.
- Library browsers can filter and search workflows by `tags`.
- Annotations are optional, so existing workflows are not affected.
- The engine ignores annotations, so they have no runtime cost.
- Strict mode (ADR-0001) may validate conventional keys, but it does not require
  any of them.

## Alternatives considered

- **Keep metadata in a separate `.meta.yml` file.** Rejected because it drifts
  from the workflow and complicates installation and versioning.
- **Put every annotation at the top-level of the workflow object.** Rejected
  because it would collide with future runtime fields and would not allow
  per-node documentation.
- **Use YAML comments for documentation.** Rejected because comments are not
  machine-readable in a structured way.
