# handl-vm-config

A sandboxed [`Host`] for the handl VM. Pure functions only — no I/O, no side effects, no `import()`. Resource limits enforced by the VM.

A host that provides only pure functions is automatically sandboxed — this is a policy decision, not a language feature.

## Included

| Namespace    | Source crate                            | Highlights                                                        |
| ------------ | --------------------------------------- | ----------------------------------------------------------------- |
| built-ins    | `handl-host`                            | `add`, `mul`, `concat`, `len`, `var`, `resolve`, `apply`, `array` |
| `math.*`     | `handl-lib-utils`                       | `abs`, `floor`, `sqrt`, `pow`, `sin`, `cos`, …                    |
| `str.*`      | `handl-lib-utils`                       | `upper`, `lower`, `split`, `join`, `replace`, …                   |
| `array.*`    | `handl-lib-utils`                       | `map`, `filter`, `reduce`, `sort`, `flatten`, …                   |
| `set.*`      | `handl-lib-utils`                       | `unique`, `union`, `intersection`, …                              |
| `bool.*`     | `handl-lib-utils`                       | `and`, `or`, `not`, `xor`, …                                      |
| `obj.*`      | `handl-lib-utils`                       | `keys`, `values`, `merge`, `deep_get`, …                          |
| `fn.*`       | `handl-lib-utils`                       | `pipe`, `compose`, `apply`, …                                     |
| `type.*`     | `handl-lib-utils`                       | `to_string`, `is_null`, `type_of`, …                              |
| `crypto.*`   | `handl-lib-crypto`                      | `sha256`, `md5`, `base64_encode`, `hex_encode`, …                 |
| `datetime.*` | `handl-lib-datetime`                    | `from_unix`, `format`, `parse`, `add_days`, …                     |
| `intl.*`     | `handl-lib-intl`                        | `format_number`, `select_plural`, `format_list`, …                |
| `kv.*`       | `handl-lib-kv` + `handl-lib-kv-hashmap` | `get`, `set`, `has`, `remove`, `clear`, `size`                    |

## Excluded (side effects)

- `handl-lib-rand` — non-deterministic RNG
- `handl-lib-platform` / `handl-lib-platform-std` — filesystem, env, HTTP, process spawning, system clock

## Resource limits

| Limit       | Default   | Description                                                 |
| ----------- | --------- | ----------------------------------------------------------- |
| `max_depth` | `256`     | Maximum scope depth (call nesting). Global scope = depth 1. |
| `max_calls` | `100_000` | Maximum function calls per evaluation.                      |

Exceeding a limit returns `Error::ResourceLimit`. Set to `None` to disable a limit.

## Usage

```rust
use ac_runtime_sandbox::{SandboxHost, SandboxLimits};
use handl_vm::Vm;

let host = SandboxHost::new(SandboxLimits::default());
let ctx = host.new_context();
let vm = Vm::new(host);
```

Custom limits:

```rust
use ac_runtime_sandbox::{SandboxHost, SandboxLimits};

let limits = SandboxLimits {
    max_depth: Some(50),
    max_calls: Some(1_000),
};
let host = SandboxHost::new(limits);
let ctx = host.new_context();
```

Add your own modules on top of the sandbox defaults:

```rust
use ac_runtime_sandbox::SandboxHost;

let host = SandboxHost::default();
let modules = host.into_modules()
    .with(MyCustomModule);
```
