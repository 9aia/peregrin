# Why the workspace has many small crates

The workspace has 40+ crates. This is deliberate. Each crate exists because it has a distinct dependency boundary, a distinct consumer, or a distinct compilation reason.

## The layering

```text
Language surface          Parsing & AST
─────────────             ─────────────
dialect                   dialect-canonical    frontend-json/yaml/toml    frontend    source    ast    fields

Compiler pipeline         Type system
──────────────────        ────────────
compiler-core             typecheck-core
compiler-pass-parse       typecheck-infer
compiler-pass-extract     typecheck-key-expr / key-arg / key-merge / key-inherit / key-mutable
compiler-pass-resolve     typecheck-import
compiler-pass-typecheck   typecheck-schema-json
compiler-pass-generate
compiler                  compiler-tests

Runtime                   Libraries (host-provided modules)
───────                   ──────────────────────────────────
runtime                   runtime-core         lib-utils (math, string)
vm                        lib-kv + lib-kv-hashmap
                         lib-platform + lib-platform-std

Module resolution         Type generation
──────────────────        ────────────────
module-resolver-core      typegen-core
module-resolver-memory    typegen-key-set-special
module-resolver-noop      typegen-import
module-resolver-file      typegen-backend-json-schema
module-resolver-virtual
module-resolver-http
module-resolver-git

Integration               Tooling
───────────               ───────
host                      cli
                          bench
```

## Why split

### 1. Dependency boundaries

`lib-platform` defines abstract service traits (`Fs`, `Env`, `Path`, `Time`, `HttpClient`, `Cmd`) with zero I/O dependencies. `lib-platform-std` provides concrete implementations backed by `std::fs`, `std::env`, `ureq`, etc. If you only need the traits (e.g. for testing or an alternative backend), you depend on `lib-platform` alone and never pull in `ureq` or `std::process`.

Similarly, `lib-kv` defines the key-value store trait; `lib-kv-hashmap` provides the `HashMap` implementation. Consumers that only need the trait don't pay for the implementation's dependencies.

### 2. Compiler pass isolation

Each compiler pass (`parse`, `extract`, `resolve`, `typecheck`, `generate`) is a separate crate. This means:

- A pass can only depend on its inputs and `compiler-core`, not on later passes.
- The dependency graph enforces pipeline ordering — you cannot accidentally import `pass-generate` from `pass-parse`.
- Each pass can be tested and benchmarked in isolation.

### 3. Type system modularity

The typechecker is split into focused crates:

- **`typecheck-core`** — shared types and the inference engine interface.
- **`typecheck-infer`** — type inference algorithm.
- **`typecheck-key-*`** — one crate per key-handling concern (expression inference, argument checking, merge semantics, inheritance, mutability).
- **`typecheck-import`** — import-specific type checking.
- **`typecheck-schema-json`** — JSON Schema backend for type validation.

This lets a consumer depend on only the typechecking pieces they need. A tool that only checks key expressions doesn't pull in import resolution or schema validation.

### 4. Module resolver pluggability

`module-resolver-core` defines the `ModuleResolver` trait. Six implementations live in separate crates: `memory`, `noop`, `file`, `virtual`, `http`, `git`. A CLI might depend on `file` and `git`; an embedded runtime might use only `memory`. The `git` resolver pulls in `git2` — if you don't need it, you don't pay for it.

### 5. Frontend format isolation

`frontend-json`, `frontend-yaml`, and `frontend-toml` each pull in their respective parsing crates (`serde_json`, `serde_yaml`, `toml`). `frontend` aggregates them. A consumer that only reads JSON doesn't need `serde_yaml`.

### 6. Runtime vs. VM vs. runtime-core

- **`runtime`** — `NonFunctionValue`, the JSON-compatible value type. No execution logic.
- **`vm`** — the evaluator: `Context`, `Function` trait, `Error` type. Depends on `runtime` and `ast`.
- **`runtime-core`** — `Module` trait and `ModuleHost` (the registry). Depends on `vm` but not on any library crate.

This separation means a library crate (`lib-utils`, `lib-platform`) depends on `runtime-core` for the `Module` trait without pulling in the evaluator. The evaluator can be swapped without touching module definitions.

### 7. Test-only crates

`compiler-tests` is a dedicated integration-test crate that depends on all compiler passes and typecheck crates. It exists so the pass crates themselves stay lean — their unit tests cover internals, while end-to-end pipeline tests live in one place with full dependency visibility.

## The principle

Every crate boundary corresponds to a real dependency or consumer boundary. If two crates always depend on each other and are always consumed together, they should merge. If one can be used without the other, they should stay split.
