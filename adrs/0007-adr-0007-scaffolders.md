+++
id = "ADR-0007"
title = "ADR-0007: Scaffolders"
status = "open"
priority = "medium"
+++

# ADR-0007: Scaffolders

> **Platform note:** Peregrin is an orchestration plugin built on handl.
> handl provides the language, typechecker, module system, standard library,
> formatter, linter, package manager, and HMR. This ADR covers Peregrin-specific
> orchestration concerns only. Language-level features referenced here are
> defined in `packages/handl/project/adrs/`.


## Status

Proposed

## Context

Peregrin workflows, drivers, and libraries are growing. New users and projects
need a fast way to start from a known good structure rather than copying files
by hand. At the same time, the project wants to ship reusable patterns
(libraries) that can be instantiated for a specific repository or team.

Two commands scaffold starting points from templates. `handl create --template`
creates a new project or workflow from an opinionated template. `handl generate`
scaffolds a single artifact into an existing project. Both are distinct from
`handl export` (ADR-0006), which renders artifacts from an existing workflow.

## Decision

Peregrin will provide `handl create --template` and `handl generate` commands backed
by a shared template system. `handl create` is for one-shot project or workflow
creation; `handl generate` is for adding artifacts to an existing project.

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
handl create my-campaigns --template project
handl create deploy --template workflow --pattern dependency-upgrade

handl generate workflow --name deploy
handl generate workflow --pattern dependency-upgrade
handl generate driver --name httpbin
handl generate observer --name datadog
handl generate store --name postgres
handl generate project --name my-campaigns
```

Templates are resolved from a built-in template directory, a user template
directory, or a remote library via `include:`.

## Consequences

- New users can get a working workflow in one command.
- Library authors can ship templates alongside reusable patterns.
- The CLI needs a template engine that substitutes variables (`{{name}}`,
  `{{driver}}`, etc.) into static files.
- `handl generate` and `handl create` must not conflict with `handl export`. `handl create`
  and `handl generate` write new source; `handl export` renders existing source.
- Templates depend on ADR-0010 (superseded by handl ADR-0012) (Versioning) to stamp `version` into generated
  files and track the template version in a lockfile.
- Scaffolders can include default `annotations` from ADR-0009 (superseded by handl ADR-0011) in generated
  workflows and nodes.

## Alternatives considered

- **Use `handl generate` for both export and scaffolding.** Rejected because the
  two operations have different audiences and different outputs. Combining them
  would make the `generate` command confusing.
- **Provide only `handl init`.** Rejected because `handl init` is only for new
  projects. Workflow, driver, and observer templates need a broader command
  surface.
