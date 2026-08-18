# Data Types (Digest)

- Branded types with `Brand` and `Schema.Brand` for domain primitives.
- `Option` for absent values. `Either` for sync result/error. No `null`/`undefined` in APIs.
- Use `DateTime` and `Duration` from `effect` for time.
- Use `HashMap`/`HashSet` from `effect` for fast lookups.
- Define `Schema` for all data crossing a boundary. Decode explicitly.
- Use `Redacted` for secrets.
