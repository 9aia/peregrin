+++
id = "ADR-0010"
title = "ADR-0010: Versioning for workflows, snippets, and nodes"
status = "open"
priority = "medium"
+++

# ADR-0010: Versioning for workflows, snippets, and nodes

## Status

Superseded by [handl ADR-0012](../../packages/handl/project/adrs/0012-versioning.md). Versioning is now a language platform concern.

Proposed

## Context

The install and registry system (ADR-0008) and the scaffolders (ADR-0007) need
a way to identify and reuse specific versions of reusable workflows, patterns,
and nodes. Without versions, `handl install org/repo` cannot know which revision to
fetch, and `include:` cannot reliably reference a pattern that may change over
time. Similarly, scaffolders cannot generate lockfiles or track which template
version a project was created from.

A `version` field must be a first-class concept in the YAML so that versioning
works the same way for local files, installed packages, and generated artifacts.

## Decision

Peregrin will support an optional `version` field on `Workflow`, `Node`, and
`EventSchema`. The field is a string that follows a simple semver-compatible
format. It is used by the registry, the installer, the file router, and the
scaffolders to identify and pin reusable units.

### Example

```yaml
workflow: deploy
version: "1.2.0"

include:
  - pattern: 9aia/dependency-upgrade@1.2.0

nodes:
  - id: tests
    version: "1.0.0"
    driver: shell
    on: [pr.opened]
    emits: [build.done, build.failed]
    params:
      cmd: "npm test"

  - id: upgrade
    driver: runner
    version: "1.2.0"
    on: [start]
    params:
      workflow: dependency-upgrade
```

### Model changes

- Add `version: string` to `Workflow`, `Node`, and `EventSchema`.
- Default `version` is `"0.0.0"` for backwards compatibility and is only
  serialized if it differs from the default or if other version-related fields
  are present.
- `Node.toDict` and `Workflow.toDict` include `version` when it is non-default.

### Specifier grammar

The install and `include` systems can use a versioned specifier:

```text
<name-or-path>[@<version>]
```

Examples:

```bash
handl install 9aia/deploy-patterns
handl install 9aia/deploy-patterns@v1.2.0
handl install gitlab:acme/campaigns@1.0.0

include:
  - pattern: 9aia/deploy-patterns@1.2.0
  - pattern: ./local/patterns/upgrade@2.0.0
```

A missing version uses the latest resolved tag or the default branch. An exact
version pins the package or file.

### Lockfile

`handl install` writes a lockfile at `.peregrin/lock.json`:

```json
{
  "packages": {
    "9aia/deploy-patterns@1.2.0": {
      "resolved": "https://github.com/9aia/deploy-patterns",
      "ref": "v1.2.0",
      "sha": "abc123...",
      "into": ".peregrin/packages/9aia/deploy-patterns"
    }
  }
}
```

The lockfile makes `handl install` reproducible without checking installed
packages into version control.

### Integration with other ADRs

- **ADR-0007 (Scaffolders).** Generated workflows and projects carry a
  `version` field. The `handl create` command can stamp the current scaffold
  version into the lockfile.
- **ADR-0008 (Install and registries).** Registries resolve versions by tag,
  branch, commit, or path. The `handl install` command uses the lockfile to pin
  exact versions.
- **ADR-0009 (Annotations).** `since` and `deprecated` annotations can refer to
  `version` values.

## Consequences

- Reusable patterns can be versioned and pinned.
- `include:` can reference stable versions of installed or remote packages.
- `handl install` becomes deterministic when a lockfile is present.
- The file router must be able to load a workflow from a versioned install path.
- The `Workflow` and `Node` models need a `version` field and parsing logic.
- Scaffolders can generate `version` into new projects.

## Alternatives considered

- **Version only the package, not the individual node.** Rejected because a
  workflow may want to version its own nodes separately when the workflow is
  large and some nodes evolve faster than others.
- **Use git commit SHAs as versions.** Rejected because SHAs are not human
  readable and do not express compatibility. They can still appear in the
  lockfile, but the public specifier uses semver-like tags.
- **Put version in the filename.** Rejected because it makes `include:` verbose
  and harder to update. A field is cleaner.
