+++
id = "ADR-0019"
title = "ADR-0019: Rich capability policy model"
status = "open"
priority = "medium"
+++

# ADR-0019: Rich capability policy model

> **Platform note:** Peregrin is an orchestration plugin built on handl.
> handl provides the language, typechecker, module system, standard library,
> formatter, linter, package manager, and HMR. This ADR covers Peregrin-specific
> orchestration concerns only. Language-level features referenced here are
> defined in `packages/handl/project/adrs/`.


## Status

Proposed

## Context

The `31-capability-confinement.yaml` example demonstrates capability safety by denying all shell commands except `echo *`. This is a good start, but the policy model is limited to a shell glob allowlist. As workflows use `agent`, `http`, `python`, and other drivers, capabilities need to cover more dimensions: HTTP hosts and methods, agent permission modes, Python module imports, and file-system paths.

The current check is also runtime-only. A missing or incorrect capability is only detected when the node runs. In production, it is better to catch policy violations before the workflow starts.

## Decision

Expand the `capabilities` block to support per-driver allow/deny rules with patterns, and validate policies at `validate` time.

```yaml
capabilities:
  default_policy: deny
  shell:
    allow: ["echo *", "cat *"]
    deny: ["rm *", "* > *"]
  http:
    allow:
      - method: GET
        host: "api.github.com"
      - method: POST
        host: "api.github.com"
        path: "/repos/*/issues/*/comments"
  agent:
    allow:
      - runner: devin
        permission_mode: safe
  python:
    allow:
      - module: "json"
      - module: "re"
  fs:
    read: ["${config.worktree_base}/*"]
    write: []
```

Rules are evaluated in order: explicit `deny` overrides `allow`, and `default_policy` is the fallback. Placeholders such as `${config.*}` and `${state.*}` are resolved at validation and runtime.

Validation at `handl validate` checks that every node's `params` values are allowed by the policy. For example, an `http` node that calls a host not in the allowlist fails validation in `strict` mode or emits a `capability.violation` at runtime otherwise.

## Consequences

- Agentic workflows can be constrained without writing shell wrappers.
- `handl validate` can catch unsafe workflows before they run.
- The capability checker must be extended to understand each driver's `params` schema.
- New driver authors must supply a capability manifest.

## Alternatives considered

- **Keep runtime-only, per-driver capability checks.** Rejected because it fails too late for production safety.
- **Use one flat allow/deny list without driver names.** Rejected because different drivers have different dimensions (hosts, methods, runners, modules).
- **Delegate all capability decisions to the operating system.** Rejected because OS-level sandboxing is environment-specific and does not give Peregrin the semantic knowledge to produce good error messages.
