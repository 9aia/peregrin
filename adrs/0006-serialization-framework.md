# ADR 0006: Serialization Framework

## Status

Accepted

## Context

Peregrin needs a uniform, format-agnostic serialization ecosystem for the
standard library. The existing language frontends (`frontend-json`, `frontend-toml`,
`frontend-yaml`, etc.) in the `ac-lang` Rust crate already parse these formats.
The serialization layer itself should live in Rust crates for performance, bound
to proven Rust libraries (`serde_json`, `toml`, `serde_yaml`, etc.).

Goals:

1. Separate the serialization framework from individual format crates.
2. Provide one shared error type and one set of core contracts: `Encoder`,
   `Decoder`, `Serialize`, and `Deserialize`.
3. Keep each format crate focused only on its wire representation.
4. Use mature Rust libraries under the hood for production throughput.

## Decision

Create Rust crates under `packages/ac-lang/crates/lib-*`.

- `ac-lib-serialize` defines `SerializeError`, `Encoder`, `Decoder`, `serialize`,
  and `deserialize`.
- `ac-lib-json` / `ac-lib-toml` / `ac-lib-yaml` implement the framework traits for
  their respective `Value` types, using `serde_json`, `toml`, and `serde_yaml`.
- `Json` / `Toml` / `Yaml` combined codecs are exposed alongside split
  `*Encoder` / `*Decoder` types.
- Format crates are added as workspace members in `packages/ac-lang/Cargo.toml`.

## Consequences

- All serialization code in the standard library shares one Rust error channel.
- Adding a new format is a matter of adding a `crates/lib-<format>` crate and
  implementing `Encoder` / `Decoder` for it.
- The current format crates are pure Rust and bind directly to high-performance
  libraries, with no intermediate TypeScript layer.
