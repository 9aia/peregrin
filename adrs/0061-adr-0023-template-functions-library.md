+++
id = "ADR-0061"
title = "ADR-0023: Template functions library"
status = "open"
priority = "medium"
+++

# ADR-0023: Template functions library

## Status

Superseded by [handl ADR-0018](../../packages/handl/project/adrs/0018-stdlib-functions.md). Standard library functions are now a language platform concern.

Proposed

## Context

The template engine resolves `${...}` placeholders inside node `params` values before a driver runs. Today the supported placeholders are limited to value lookups (`${event.field}`, `${config.key}`, `${secret.name}`, `${node.id}`, `${state.key}`) and two helpers, `file` and `shell`, are imported but only exposed through a narrow whitelist.

Workflow authors frequently need small utility values inside templates:

- A unique idempotency key for an HTTP request (`${uuid()}`).
- A random choice or jitter value (`${random(1, 10)}`).
- A timestamp for a payload (`${now()}` or `${now.iso()}`).
- A file's contents inserted inline (`${file("/tmp/data.txt")}`).
- A shell command's output captured safely (`${shell("git rev-parse HEAD")}`).
- An environment variable with a default (`${env("HOME")}` or `${env("REGION", "us-east-1")}`).

Without these, authors either precompute values in shell nodes or leak imperative logic into otherwise declarative workflows. At the same time, the engine must not become an arbitrary code execution channel. Any function support must be explicit, whitelisted, and auditable.

## Decision

Add a curated template functions library to `handl-template-engine`.

- Functions are invoked with the existing `${...}` syntax: `${name(arg1, arg2)}`.
- A function is only resolved if its name is in an explicit whitelist passed to the template resolver.
- Built-in functions are implemented as TypeScript bindings. They are reserved for performance-critical or security-sensitive operations where a YAML implementation would be unsafe or slow.
- The first built-in functions are:
  - `${random()}` / `${random(min, max)}` — a random number. Marked as non-deterministic.
  - `${uuid()}` — a v4 UUID. Marked as non-deterministic.
  - `${now()}` / `${now.iso()}` / `${now.unix()}` — the current time. Marked as non-deterministic.
  - `${file(path)}` — read a file from the workflow workspace.
  - `${shell(command)}` — run a shell command and capture `stdout`.
  - `${env(name, default)}` — read an environment variable, with an optional default.
- The resolver tracks whether a value is deterministic. If any non-deterministic function is used, the containing node is marked as non-replayable and the workflow's effective `replayable` flag becomes `false` (see ADR-0027).
- In strict mode, functions that access the filesystem, run shell commands, or read the environment require explicit capability grants.

## Consequences

- Templates can express more logic without leaving the declarative layer.
- The engine surface stays safe because only whitelisted functions are evaluated.
- Built-in functions keep hot paths fast and avoid YAML interpreter overhead.
- Workflows that use `random`, `uuid`, or `now` cannot be replayed deterministically unless the runner seeds or mocks those functions.
- Capability checking becomes more important: `file`, `shell`, and `env` are security-sensitive.

## Alternatives considered

- **Arbitrary JavaScript expressions inside `${...}`.** Rejected. It would make the engine powerful but unsafe and hard to audit.
- **Pipe-only syntax (`${value | fn}`).** Rejected. Functions like `random()` and `uuid()` have no input value, so the call syntax is clearer.
- **Implement every function as a separate package.** Rejected. A single `handl-template-engine` library is simpler and the registry can still delegate to drivers later.
