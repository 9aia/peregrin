+++
id = "ADR-0020"
title = "ADR-0020: Package registry and local DX"
status = "open"
priority = "medium"
+++

# ADR-0020: Package registry and local DX

> **Platform note:** Peregrin is an orchestration plugin built on handl.
> handl provides the language, typechecker, module system, standard library,
> formatter, linter, package manager, and HMR. This ADR covers Peregrin-specific
> orchestration concerns only. Language-level features referenced here are
> defined in `packages/handl/project/adrs/`.


## Status

Proposed

## Context

The `packages/handl-examples/examples/lib/` directory contains reusable node YAMLs, but the README tells users to copy them into `.handl/lib/`. This is not dependency management. The example header comments also require `node apps/cli/dist/cli.mjs validate ...` instead of a first-class `handl` command, and most examples do not use `events:` schemas even though `handl typegen` exists.

These friction points block a healthy ecosystem and make the tool harder to learn.

## Decision

Improve the package and local-development experience in three areas.

### 1. Node and workflow packages

Add an `handl install` or `handl add <package>` command that fetches reusable node libraries into `.handl/lib/`. A package is a directory of `*.node.yml`, `*.snippet.yml`, and `*.event.json` files plus a package manifest. The file router continues to auto-discover them, but the source is now versioned and sharable.

```bash
handl add handl-lib-git
handl add handl-lib-test
```

Workflows can also declare dependencies in the workflow file:

```yaml
import:
  - handl-lib-git
  - handl-lib-test
```

### 2. Stronger static validation

`handl validate` should catch:
- Events emitted by a node that are not declared in `emits`.
- Missing secrets referenced in `params`.
- Unknown placeholders in `params` when `strict: true`.
- Payload fields that do not match `events:` or `*.event.json` schemas.
- Capability violations (see ADR-0019).

### 3. First-class CLI commands

Replace the `node apps/cli/dist/cli.mjs` invocations in example headers with `handl` commands:

```bash
handl validate examples/00-foundations/00-hello.yaml
handl run --dry-run examples/20-real-world/20-deploy-pipeline.yaml
handl typegen workflow.yaml
handl explain workflow.yaml
```

`handl explain` should print the inferred event graph, ready nodes, and fan-in/fan-out structure to help authors debug topology.

## Consequences

- Reusable nodes can be distributed, versioned, and updated without copy-paste.
- Examples become safer and more self-documenting through schemas and simpler commands.
- The CLI surface grows and needs to remain stable.
- A package index or registry may be desirable later, but the first version can support local and git-based packages.

## Alternatives considered

- **Use npm as the package manager for node libraries.** Rejected because npm packages are overkill for plain YAML fragments and would require a `node_modules` layout.
- **Keep `include:` as the only sharing mechanism.** Rejected because it does not support versioning and pushes composition into a single large file.
- **Do not add a first-class CLI; rely on scripts in `package.json`.** Rejected because it ties the tool to the monorepo and is not discoverable for end users.
