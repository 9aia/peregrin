+++
id = "ADR-0071"
title = "ADR-0031: Peregrin CLI `lint` subcommand"
status = "open"
priority = "medium"
+++

# ADR-0031: Peregrin CLI `lint` subcommand

## Status

Superseded by [handl ADR-0015](../../packages/handl/project/adrs/0015-linter.md). The linter is now a language toolchain concern (`handl lint`), not Peregrin-specific (`pereg lint`)

Proposed

## Context

`handl validate` checks that a workflow loads and that basic structural invariants hold: every node has an `id` and `driver`, declared `emits` are consistent, and so on. As the language and standard library grow, there are more conventions that are not strictly load errors but do affect quality, maintainability, and safety:

- Library nodes should carry `annotations` (`title`, `description`, `author`, `since`, `see`) per ADR-0029.
- Nodes that call external drivers should declare `timeout`, `retry`, or `circuit_breaker`.
- Node `id`s and event `type`s should follow the dotted namespace convention.
- Workflow files should be formatted consistently (ADR-0030).
- Some fields are deprecated or discouraged in favor of newer equivalents.

These checks are too opinionated for `validate`, which must stay focused on "can the engine run this file?". They are also too Peregrin-specific for a generic YAML linter such as `yamllint` or `oxlint`. A dedicated `lint` subcommand is the natural place for convention and best-practice checks.

## Decision

Add a `lint` subcommand to the Peregrin CLI, invoked as `pereg lint <workflow...>`.

`pereg lint` loads the workflow with the same path used by `validate` and `typegen`, then runs a rule set against the loaded model. It reports issues with a rule id, severity, message, and file location. Auto-fixable issues can be corrected with `--fix`.

### Command shape

```bash
pereg lint workflow.yaml                    # report issues, exit non-zero on errors
pereg lint workflow.yaml --fix              # apply safe fixes in place
pereg lint workflow.yaml --fail-on-warn     # exit non-zero on warnings too
pereg lint workflow.yaml --except style     # skip the style rule
pereg lint libraries/**/*.yaml              # lint many files
```

### Severity levels

- `error` — the workflow violates a hard convention and must not pass CI.
- `warn` — the workflow works but is missing a recommended practice.
- `info` — a reminder or style suggestion.

By default `pereg lint` exits `0` when only warnings and info are present and `1` when any `error` is present. `--fail-on-warn` makes warnings fail the command too.

### Example rules

The rule set is extensible. The initial rules cover the conventions introduced by ADR-0029 and ADR-0030:

- `naming-convention` (`error`): a node `id` or event `type` does not follow the dotted `<domain>.<group>.<name>` pattern.
- `missing-annotations` (`warn`): a `.node.yml` or `.snippet.yml` preset is missing one of `title`, `description`, `author`, `since`, `see`.
- `missing-resilience` (`warn`): a node that runs an external driver lacks `timeout`, `retry`, or `circuit_breaker`.
- `no-undeclared-event` (`error`): a node subscribes to an event type that no node or `events` entry emits.
- `missing-emits` (`warn`): a node does not declare an `emits` list.
- `unused-snippet` (`info`): a `snippet` is defined but never referenced.
- `deprecated-field` (`warn`): a field has been superseded by another and should be migrated.
- `style` (`warn`, auto-fixable): the file is not in the canonical format defined by ADR-0030.
- `strict-recommended` (`info`): a workflow with external or fallible drivers is missing `strict: true`.

### Issue format

Each rule returns a structured issue:

```ts
type LintIssue = {
  rule: string;
  severity: "error" | "warn" | "info";
  file: string;
  line?: number;
  message: string;
  fix?: (content: string) => string;
};
```

### Implementation

- Add the `lint` command to `apps/cli/src/main.ts` next to `run`, `validate`, `typegen`, and `fmt` (ADR-0030).
- The loader returns the parsed `Workflow` plus source locations when the parser supports them.
- Each rule is a pure function from `Workflow` and file metadata to a list of `LintIssue`s.
- `Effect` is used to collect all rules and return a final exit code.
- `--fix` applies fixes in a stable order and only for `auto-fixable` issues such as `style`. Rules that could change semantics require the user to edit the file manually.
- The `style` rule reuses the same formatter module that `pereg fmt` uses.

## Consequences

- CI can enforce Peregrin conventions without making `validate` itself opinionated.
- Library authors get clear, rule-based feedback on naming, annotations, and resilience.
- The rule set can grow incrementally as new ADRs add conventions.
- Auto-fix reduces the friction of adopting style rules.
- Keeping `validate` and `lint` separate preserves `validate` as a fast, load-only check while `lint` becomes a slower, more thorough quality gate.

## Alternatives considered

- **Fold all lint checks into `handl validate`.** Rejected. `validate` should remain focused on load-time correctness. Mixing conventions into it would make the command slower and less predictable.
- **Use `yamllint` or `oxlint` for workflow files.** Rejected. Generic linters do not understand Peregrin semantics such as dotted namespaces, `emits`, or `annotations`.
- **Implement `lint` as a standalone package.** Rejected for the same reason as ADR-0030: the CLI already owns the command surface and depends on the loader.
