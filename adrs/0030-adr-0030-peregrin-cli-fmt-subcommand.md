+++
id = "ADR-0030"
title = "ADR-0030: Peregrin CLI `fmt` subcommand"
status = "open"
priority = "medium"
+++

# ADR-0030: Peregrin CLI `fmt` subcommand

## Status

Superseded by [handl ADR-0014](../../packages/handl/project/adrs/0014-formatter.md). The formatter is now a language toolchain concern (`handl fmt`), not Peregrin-specific (`pereg fmt`)

Proposed

## Context

Peregrin workflows are authored as YAML. Over time the same logical file can be written in many different styles: extra blank lines, mixed indentation, keys in arbitrary order, trailing whitespace, or lists expanded onto many lines. This creates noisy diffs, makes reviews harder, and lets formatting arguments creep into every pull request.

The project already has `handl validate` and `handl typegen` as first-class CLI subcommands that understand the Peregrin workflow model. A formatter that is equally aware of the language can do more than a generic YAML formatter:

- It can enforce a canonical key order that matches the workflow, node, and event schemas.
- It can keep `include` and `import` blocks near the top so readers understand the file’s dependencies first.
- It can preserve or re-emit `# yaml-language-server:` schema comments that `typegen` injects.
- It can fail safely on invalid YAML without changing the file.

A generic tool such as `prettier` or `dprint` with a YAML plugin only knows YAML syntax. It does not understand Peregrin-specific ordering and is one more dependency to configure and keep in sync with language changes.

## Decision

Add a `fmt` subcommand to the Peregrin CLI, invoked as `pereg fmt <workflow...>`.

`pereg fmt` formats Peregrin workflow YAML using the same loader/emitter stack as `validate` and `typegen`. It is a read-only command by default and prints the formatted result to stdout. In-place editing and CI checking are opt-in flags.

### Command shape

```bash
pereg fmt workflow.yaml                    # print formatted workflow to stdout
pereg fmt workflow.yaml --write            # overwrite the file in place
pereg fmt workflow.yaml --check            # exit non-zero if the file is not already formatted
pereg fmt libraries/**/*.yaml --write      # format many files
```

### Canonical style

1. **Two-space indentation.**
2. **A single trailing newline** at the end of each file.
3. **No trailing whitespace** on any line.
4. **Canonical key order for mappings.** Unknown keys are kept after known keys in their original relative order.
   - Workflow top-level: `name`, `include`, `import`, `stop_on`, `events`, `snippets`, `config`, `strict`, `state`, `payload_schemas`, `nodes`.
   - Node mapping: `id`, `driver`, `handler`, `on`, `emits`, `snippet`, `mode`, `params`, `max_fires`, `until`, `retire_on`, `retry`, `circuit_breaker`, `poll`.
   - Event mapping: `type`, `fields`, `schema`.
5. **Array order is preserved.** Do not sort `nodes`, `events`, `emits`, `on`, or any other list, because authors may group items intentionally.
6. **Comments are preserved** where the underlying YAML document model supports it. If full comment round-tripping cannot be guaranteed in the first implementation, the command documents that behavior and warns the user.

### Implementation

- Add the `fmt` command to `apps/cli/src/main.ts` alongside `run`, `validate`, and `typegen`.
- Use the `yaml` package already consumed by `handl-loader-yaml` to parse and stringify YAML.
- Reuse `loadRaw` or an equivalent loader path so that `fmt` only sees the file being edited, not files merged through `include`.
- The formatter operates on the parsed document, re-orders mapping keys with a small schema-aware sorter, then stringifies with the canonical rules.
- `--check` compares the emitted text to the original and exits `1` when they differ, but does not write anything.
- `--write` overwrites the source file atomically.

## Consequences

- All Peregrin workflow files can be formatted the same way, reducing style noise in diffs.
- `pereg fmt --check` can run in CI to enforce formatting without requiring a separate tool.
- The formatter stays in sync with the Peregrin loader, so language changes can update the canonical style in one place.
- Existing workflow files will need a one-time pass with `pereg fmt --write` before CI enforcement is enabled.
- YAML comments may not round-trip perfectly in the first version; this should be made explicit until the parser/emitter fully supports them.

## Alternatives considered

- **Use `prettier` with the YAML plugin.** Rejected. It does not know Peregrin-specific key order and adds a separate configuration file to keep in sync.
- **Use `dprint` or `yamlfmt`.** Rejected for the same reason: generic YAML formatters cannot encode the semantic ordering of the workflow language.
- **Implement `fmt` as a standalone package.** Rejected. The CLI already imports `handl-loader-yaml`, `handl-core`, and the `yaml` parser; a separate package would duplicate dependencies and the command surface.
- **Make the formatter edit files by default.** Rejected. Printing to stdout by default is safer and matches the behavior of many formatters (`gofmt`, `terraform fmt` without flags).
