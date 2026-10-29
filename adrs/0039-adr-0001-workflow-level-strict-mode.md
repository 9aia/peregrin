+++
id = "ADR-0039"
title = "ADR-0001: Workflow-level strict mode"
status = "open"
priority = "medium"
+++

# ADR-0001: Workflow-level strict mode

> **Platform note:** Peregrin is an orchestration plugin built on handl.
> handl provides the language, typechecker, module system, standard library,
> formatter, linter, package manager, and HMR. This ADR covers Peregrin-specific
> orchestration concerns only. Language-level features referenced here are
> defined in `packages/handl/project/adrs/`.


## Status

Proposed

## Context

Peregrin workflows use event payloads, secrets, config, and runtime values as the source of truth for driver inputs. By default, the template engine, the payload validation, and the driver environment are permissive:

- An unknown placeholder in a `params` field is left for the shell to interpret.
- A missing event payload field silently becomes an empty string.
- A missing schema does not reject a payload.
- A workflow that receives extra events or uses an unlisted driver is not necessarily an error.

This is good for exploratory work and for writing shell scripts where `${VAR}` is common. It is dangerous in production, where a typo, a missing secret, or an unexpected payload shape should fail fast rather than produce an empty string or run the wrong command.

Many users want a single switch to turn the engine from permissive into strict. Other workflow tools have a similar concept: Terraform has `-var` defaults vs strict validation, GitHub Actions has `continue-on-error`, and CI systems have `set -e`. The need is not to change the language of the workflow, but to change the default assumptions.

## Decision

Introduce a `strict` boolean field on the `Workflow` model. The default is `false` to keep existing permissive behaviour. When `strict: true`, the engine will apply a strict policy set.

The `strict` field activates the following behaviours:

1. **Required schemas.** Every event listed in `events` must have a payload schema defined. Payloads must validate against the schema before the event is accepted by the runner. A validation failure is a runtime error.
2. **Deny by default.** Every node must declare an explicit `emits` list. If a driver returns an event type not declared in `emits`, the runner rejects it. If a node fires without declaring `on` or `emits`, the runner treats it as an error.
3. **Unknown placeholder rejection.** Any `${...}` expression in `params` values that the template engine cannot resolve becomes a `TemplateError` instead of being left untouched or rendered as an empty string.
4. **Missing secrets rejection.** If a `params` block references a secret (e.g. via `params.secrets` or `ctx.getSecret`) that is not present, the runner fails instead of returning an empty string.
5. **Unknown driver / handler rejection.** If the workflow references a driver or handler that is not registered, the runner fails immediately.
6. **No implicit defaults for safety-critical fields.** Capabilities and resources are not auto-inferred; they must be explicitly declared, and any operation outside the declared capability set fails.

The `strict` field is stored on `Workflow` and is propagated to the runner configuration. Individual nodes can still override the strictness of specific behaviours with explicit fields, but the default is determined by the workflow-level flag.

## Consequences

- Workflows with `strict: true` are safer and easier to audit, but require more explicit declarations.
- Workflows with `strict: false` (the default) remain easy to write and remain compatible with the current permissive behaviour.
- The `EventSchema` and `Node` model need validation helpers that can be called in both `strict` and non-strict modes.
- The `Runner`, `Bus`, and template engine need to receive the `strict` flag and branch accordingly.
- CLI and tests will need a way to toggle `strict` per run (e.g. `--strict`, or by reading the workflow field).

## Alternatives considered

- **Per-node strict flags.** More flexible, but puts the burden on every node. A workflow-level default is simpler and more discoverable.
- **Strict as the only mode.** Would break the current permissive examples and the shell-script use case. Defaulting to `false` keeps the learning curve low.
- **Multiple named modes (e.g. `mode: strict|permissive`).** A boolean is enough for the first version and can be replaced later if more modes are needed.
