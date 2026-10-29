+++
id = "ADR-0063"
title = "ADR-0025: Migrating `params` to `params`"
status = "open"
priority = "medium"
+++

# ADR-0025: Migrating `params` to `params`

## Status

Superseded by [handl ADR-0020](../../packages/handl/project/adrs/0020-parameter-naming.md). Parameter naming is now a language convention.

Accepted

## Context

Every node has a parameter map that is passed to its driver. In the YAML format and the `Node` model this field is called `params`. The name is ambiguous: it does not describe what the field contains, and it does not read well in template expressions.

We are adding driver presets and user-defined functions. Both need a clean way for users to pass values in and for preset authors to reference those values in templates. `params` is clearer in both roles:

- `driver: devin` followed by `params:` says exactly what is being sent to the driver.
- `${params.task}` in a template is unambiguous: it is a user-supplied parameter.
- It aligns with common tooling (OpenAPI, RPC, CLI frameworks) where `params` means "arguments to an operation".

The existing `${config.X}` namespace in `handl-template-engine` refers to workflow-level configuration, not node parameters. A separate `${params.X}` namespace avoids overloading `config`.

## Decision

Rename the parameter map from `params` to `params` in all user-facing YAML and in the TypeScript model.

- In `.node.yml` and `.node.yaml` files, the only key is `params`. `params` is removed.
- In driver preset files (`*.driver.yml`), the only key for the preset's default parameter map is `params`.
- In function files (`*.fn.yml`), the declaration of accepted inputs and the function body are both under `params`.
- In the template engine, the merged parameter map is exposed as `params`. `${params.task}` resolves to the user-supplied or default value. Workflow-level configuration remains `${config.X}`.
- The TypeScript `Node` model renames `params` to `params`. The loader rejects `params` with a clear error.
- The migration is a breaking change in the YAML format. All examples, tests, documentation, and templates are updated to `params` at the same time this ADR is implemented.

## Consequences

- The user-facing vocabulary is clearer. `params` looked like a preposition; `params` is a noun with an obvious meaning.
- Existing workflows that use `params` must be migrated. No compatibility alias is retained.
- Driver presets and functions can reference user input as `${params.X}`, which is distinct from workflow `${config.X}`.
- The `Node` model, loader, and all examples need updates.
- Tests must cover the new `params` key and the error message for the removed `params` key.

## Alternatives considered

- **Keep `params` and add `params` as a parallel concept.** Rejected. Two names for the same data is worse than one. Either `params` or `params` should own the parameter map.
- **Use `args` or `input` instead of `params`.** Rejected. `params` is more common in API and workflow tooling, and `args` is strongly associated with shell commands.
- **Keep `params` as a deprecated alias during a transition window.** Rejected. This is a pre-1.0 project with a small example base. A clean breaking change is cheaper than carrying the alias indefinitely.
