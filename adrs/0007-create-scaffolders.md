# ADR-0007: Create scaffolders

## Status

Proposed

## Context

Peregrin workflows, drivers, and libraries are growing. New users and projects
need a fast way to start from a known good structure rather than copying files
by hand. At the same time, the project wants to ship reusable patterns
(libraries) that can be instantiated for a specific repository or team.

A `create` or `init` command scaffolds these starting points from templates.
This is distinct from the `ac generate` command (ADR-0006), which exports
artifacts from an existing workflow. `ac create` produces new source; `ac
generate` renders existing source into another format.

## Decision

Peregrin will provide an `ac create` command and a template system for
scaffolding new projects and workflows.

### Templates (proposed)

1. **Empty workflow**
   A minimal `workflow.yaml` with a `start` event and a `hello` shell node.

2. **Workflow with pattern**
   A workflow that `include`s a built-in library pattern and wires one `runner`
   node.

3. **Custom driver**
   `src/driver.ts`, `package.json`, `tsconfig.json`, and a tiny test file, ready
   for `vp pack`.

4. **Custom observer**
   Same structure as a driver, but implementing the `Observer` adapter port.

5. **Custom store**
   Same structure, but implementing the `EventStore` adapter port.

6. **Project / workspace**
   A root with `peregrin.yaml`, a `workflows/` directory, a `patterns/`
   directory, and a GitHub Actions stub.

### Command surface (proposed)

```bash
ac create workflow --name deploy
ac create workflow --pattern dependency-upgrade
ac create driver --name httpbin
ac create observer --name datadog
ac create store --name postgres
ac create project --name my-campaigns
```

Templates are resolved from a built-in template directory, a user template
directory, or a remote library via `include:`.

## Consequences

- New users can get a working workflow in one command.
- Library authors can ship templates alongside reusable patterns.
- The CLI needs a template engine that substitutes variables (`{{name}}`,
  `{{driver}}`, etc.) into static files.
- `ac create` must not conflict with `ac generate`. `create` writes new source;
  `generate` renders existing source.
- Templates are versioned with the project so they do not drift from the current
  package API.

## Alternatives considered

- **Use `ac generate` for both export and scaffolding.** Rejected because the
  two operations have different audiences and different outputs. Combining them
  would make the `generate` command confusing.
- **Provide only `ac init`.** Rejected because `ac init` is only for new
  projects. Workflow, driver, and observer templates need a broader command
  surface.
