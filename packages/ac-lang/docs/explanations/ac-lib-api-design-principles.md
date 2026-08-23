# AC library API design principles

These principles guide the public API of the `ac-lang` facade and the `ac-*` library crates.

- **Modularity.** Functionality is split into focused crates with narrow, stable boundaries. Each crate exposes one thing well, and the facade composes them behind feature flags.

- **Type-safety.** Errors are surfaced as types, not strings. Every fallible operation returns a `Result` with a dedicated error type, and types encode invariants at the boundary.

- **Composability.** Small, single-purpose types compose through traits and modules. Higher-level APIs are built from lower-level primitives, and hosts can swap implementations without rewriting call sites.

- **Tree-shakeability.** Each major subsystem lives behind a Cargo feature and is re-exported conditionally. Disable default features and enable only what your host uses; unused crates and symbols are not compiled or linked.

- **Runtime-agnostic core.** The AST, schema, and VM know nothing about I/O, formats, or control flow. The host provides all behavior through traits, so the library runs anywhere the host can be implemented.

- **Feature-gated re-exports.** The `ac-lang` facade mirrors the crate graph with feature flags. No feature means no dependency, so hosts pay only for the capabilities they opt in to.

- **Ergonomics.** Low-level crates expose the full API; `ac-lang` provides a `prelude` and grouped re-exports for the common case. Hosts that need control can import from individual crates directly.

- **Backward compatibility.** Public types are versioned and additive. New functionality is added behind new features or new modules; existing paths are not re-interpreted.

- **Predictable allocation.** Hot paths accept borrowed data and return owned results only when necessary. Callers can reuse buffers and control when values are materialized.
