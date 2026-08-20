# ADR-0020: Package registry and local DX

## Status

Proposed

## Context

The `packages/ac-examples/examples/lib/` directory contains reusable node YAMLs, but the README tells users to copy them into `.ac/lib/`. This is not dependency management. The example header comments also require `node apps/cli/dist/cli.mjs validate ...` instead of a first-class `ac` command, and most examples do not use `events:` schemas even though `ac typegen` exists.

These friction points block a healthy ecosystem and make the tool harder to learn.

## Decision

Improve the package and local-development experience in three areas.

### 1. Node and workflow packages

Add an `ac install` or `ac add <package>` command that fetches reusable node libraries into `.ac/lib/`. A package is a directory of `*.node.yml`, `*.snippet.yml`, and `*.event.json` files plus a package manifest. The file router continues to auto-discover them, but the source is now versioned and sharable.

```bash
ac add ac-lib-git
ac add ac-lib-test
```

Workflows can also declare dependencies in the workflow file:

```yaml
import:
  - ac-lib-git
  - ac-lib-test
```

### 2. Stronger static validation

`ac validate` should catch:
- Events emitted by a node that are not declared in `emits`.
- Missing secrets referenced in `with`.
- Unknown placeholders in `with` when `strict: true`.
- Payload fields that do not match `events:` or `*.event.json` schemas.
- Capability violations (see ADR-0019).

### 3. First-class CLI commands

Replace the `node apps/cli/dist/cli.mjs` invocations in example headers with `ac` commands:

```bash
ac validate examples/00-foundations/00-hello.yaml
ac run --dry-run examples/20-real-world/20-deploy-pipeline.yaml
ac typegen workflow.yaml
ac explain workflow.yaml
```

`ac explain` should print the inferred event graph, ready nodes, and fan-in/fan-out structure to help authors debug topology.

## Consequences

- Reusable nodes can be distributed, versioned, and updated without copy-paste.
- Examples become safer and more self-documenting through schemas and simpler commands.
- The CLI surface grows and needs to remain stable.
- A package index or registry may be desirable later, but the first version can support local and git-based packages.

## Alternatives considered

- **Use npm as the package manager for node libraries.** Rejected because npm packages are overkill for plain YAML fragments and would require a `node_modules` layout.
- **Keep `include:` as the only sharing mechanism.** Rejected because it does not support versioning and pushes composition into a single large file.
- **Do not add a first-class CLI; rely on scripts in `package.json`.** Rejected because it ties the tool to the monorepo and is not discoverable for end users.
