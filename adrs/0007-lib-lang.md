# ADR 0007: lib-lang — Expose the Handl language as a reusable library

## Status

Proposed

## Context

Handl (fka Astral Chronicle / ac-lang) is currently packaged as a monolithic
compiler and runtime. The lexer, parser, AST, typechecker, and compiler passes
are not exposed as a single, stable library. This makes it impossible for a
Handl program to parse, inspect, or evaluate Handl code from within Handl
itself.

Use cases that motivate a `lib-lang` façade:

1. **Handl inside Handl**: A Handl program should be able to instantiate a
   sandboxed Handl VM, choose which modules to expose, and evaluate a Handl
   expression or a small Handl source inside that sandbox.
2. **Tooling**: Formatters, LSPs, documentation generators, and test runners
   should reuse the same lexer, parser, and AST without depending on the full
   compiler or runtime workspace.
3. **Higher-level libraries**: `lib-template` (or `lib-escaping`) should not own
   expression parsing or evaluation. They should call into a public language
   API.

The question is what the public surface of that library should be, and how it
relates to the existing compiler/runtime crates.

## Decision

Introduce a top-level library `lib-lang` (crate `ac-lib-lang` under
`crates/lib-lang`). This library is the curated, public façade for everything
needed to parse, inspect, compile, and evaluate Handl source. It re-exports
stable APIs from the existing compiler and VM crates and hides unstable
internals.

`lib-lang` is organized into submodules that mirror the language pipeline:

- `lang.lexer` — tokenization and `LexerConfig` composition.
- `lang.parser` — expression and statement parsing into the canonical AST.
- `lang.ast` — the canonical AST and traversal utilities.
- `lang.typecheck` — type inference and schema validation, where stable.
- `lang.compiler` — lowering to the canonical VM format.
- `lang.vm` — a configurable, sandboxed VM instance.

### `lang.vm` and sandboxing

A Handl program can instantiate a child Handl VM through `lang.vm.sandbox`:

```
let sandbox = lang.vm.sandbox({
  modules: ["math", "str"],
  timeout: 1000,
})

let result = lang.vm.eval(sandbox, "add(1, 2)")
```

The sandbox is a full `Vm` with a user-composed `ModuleHost`. It does not share
context with the host VM unless explicitly passed. This is the supported
mechanism for "Handl inside Handl", including template evaluation and
user-defined functions.

### Boundaries

`lib-lang` does **not** include:

- `lib-template` — template parsing and rendering is a consumer of `lib-lang`
  and `lib-escaping`. It lives in its own crate and depends on `lib-lang` only
  for parsing and sandboxed evaluation.
- `lib-escaping` — context-specific escaping (HTML, URL, CSS, JS, etc.) is a
  separate abstraction that `lib-template`, `lib-http`, and other output layers
  can share. It lives in its own crate.
- `lib-utils` — generic pure utility functions (math, string transforms,
  predicates, etc.) remain in `lib-utils`. They may be exposed to a sandbox VM
  through the `lib-utils` module, but they are not part of `lib-lang`.

## Consequences

### Positive

- Handl can reason about and evaluate Handl code from within Handl.
- Tooling can depend on a narrow `lib-lang` façade instead of the entire
  compiler workspace.
- The template engine and escaping concerns can be decoupled from the language
  implementation.
- Sandboxed execution has a single, well-defined entry point (`lang.vm.sandbox`).

### Trade-offs

- `lib-lang` becomes a large façade. We must be disciplined about what is
  public and what is internal to avoid leaking unstable compiler internals.
- Maintaining a stable `lib-lang` API may slow compiler refactors. Internals
  should remain in `ac-compiler-*` / `ac-ast-*` crates; only curated, versioned
  APIs re-export through `lib-lang`.
- The `lang.vm.sandbox` API needs careful design to prevent confusion between
  host scope and sandbox scope.

## Open questions / future ADRs

- The exact domain and public API of `lib-template` (ADR 0008).
- The escaping interface and where it lives (ADR 0009).
- Whether `lib-lang` should expose the canonical expression evaluator or a
  separate sandbox-specific evaluator.

## References

- `adrs/0001-lexer-decoupling.md`
- `adrs/0002-lego-vm-composition.md`
- `adrs/0004-colorless-execution.md`
