# ADR-0028: Driver input and output schemas

## Status

Accepted

## Context

A Peregrin driver receives a `params` map and returns a list of `Event`s. Today the only validation at that boundary is the `emits` list in strict mode, which checks that a node emits declared event types but never checks the payload shape. A node can pass a number to `shell.cmd`, an object where a string is expected, or omit a required field, and the mistake is only discovered when the driver tries to consume the value. A driver can emit an event with any payload, and downstream nodes only find out about the mismatch at runtime.

Custom driver presets (ADR-0026) make the gap wider. A preset wraps a built-in driver with default `params` and is reusable across workflows, but it has no way to declare what `params` it requires or what events it produces. A workflow author who writes `driver: git.worktree` has to read the preset file or hope the `cmd` template handles every key correctly.

A first-class `input` and `output` contract on a driver would:

- document the required and optional `params`,
- validate a node's `params` before the driver runs,
- validate the payloads of events the driver emits, and
- make custom presets as safe as built-in drivers.

## Decision

Add an optional `input` and `output` contract to every driver. Built-in drivers declare it in code; custom driver presets declare it in their `.driver.yml` file; nodes can optionally override or narrow it.

### `input`

`input` is a JSON Schema describing the `params` a driver accepts. It uses the same `fields` and `schema` shorthand that `EventSchema` already provides.

### `output`

`output` is a map of event type to JSON Schema. Each key is an event type that the driver may emit; the value is the payload schema. It uses the same `fields`/`schema` shorthand and may use `*` wildcards for families such as `shard.*.done`.

### Example in a `.driver.yml` preset

```yaml
# libraries/git/worktree.driver.yml
name: git.worktree
driver: shell
input:
  fields:
    repo: string
    worktree_base: string
    feat_id: string?
    timeout: integer?
  schema:
    required: [repo, worktree_base]
output:
  common.git.worktree.ready:
    fields:
      worktree_path: string
      branch: string
      repo: string
  common.git.worktree.failed:
    fields:
      error: string
```

### Example in a `.node.yml`

A node can override or narrow the driver contract for a single use case:

```yaml
id: setup
driver: git.worktree
params:
  repo: "${config.repo}"
  worktree_base: "${config.worktree_base}"
output:
  common.git.worktree.ready:
    required: [worktree_path, branch]
```

### Built-in driver shape

Built-in drivers expose `input` and `output` as optional metadata alongside `invoke`:

```ts
export const ShellDriver: Driver = {
  name: "shell",
  input: {
    fields: {
      cmd: "string",
      timeout: "integer?",
      emit: "object?",
    },
    schema: {
      required: ["cmd"],
    },
  },
  output: {
    "cmd.done": {
      fields: {
        stdout: "string",
        rc: "integer",
      },
    },
    "node.failed": {
      fields: {
        error: "string",
        rc: "integer",
      },
    },
  },
  invoke: (node, triggers, ctx) => Effect.gen(function* () { ... }),
};
```

### Validation rules

1. **Load-time contract discovery.** `DriverRegistry` returns the driver's `input` and `output` as part of the resolved driver. Custom presets merge their contract with any contract provided by the base driver; the preset wins on conflicts.
2. **Node `params` validation.** After template resolution, the effective `params` map is validated against `input`. In strict mode a validation failure becomes a `DriverError` and the node fails. In permissive mode the runner emits an `event.invalid` describing the error but continues.
3. **Event payload validation.** Each event emitted by the driver is validated against the schema for that event type in `output`. Missing or extra properties fail according to the `strict` workflow flag.
4. **Wildcard output keys.** An event type matches the most specific key in `output` first, then a wildcard such as `shard.*.done`. If no contract matches, permissive mode allows the event; strict mode rejects it.
5. **Data shapes.** `input` and `output` use the existing `fields` and `schema` representation. `fields` is converted to a JSON Schema object before validation. Full `schema` is accepted for complex shapes.
6. **Node overrides.** A node's `input` or `output` is merged with the driver contract before validation. Node-level declarations take precedence. This lets a workflow author narrow a generic driver to a specific contract.

### Model changes

- `Driver` gains optional `input` and `output` fields.
- `Node` gains optional `input` and `output` fields for per-node overrides.
- `DriverPreset` (from ADR-0026) gains `input` and `output` fields.
- The loader normalizes `fields`/`schema` into a `compiledSchema` before the contract is stored.

## Consequences

- Drivers become self-documenting. A workflow author can read a `.driver.yml` file and know exactly what `params` to pass and what events to expect.
- Missing or mistyped `params` are caught before the driver runs, even in permissive mode as a warning.
- Strict workflows get end-to-end payload guarantees, not just type checking.
- Custom driver presets reach parity with built-in drivers for IDE completion and validation.
- Built-in drivers need contracts added. This is extra work but can happen incrementally because the fields are optional.
- The `DriverRegistry` and `Runner` must carry contract metadata. `EventSchema.compiledSchema()` is reused, so no new validation library is needed.
- Wildcard matching and node overrides add modest complexity to the contract resolver.

## Alternatives considered

- **Keep validation at the workflow level using `payload_schemas` and `events`.** Rejected. That forces every workflow to duplicate the driver contract. The driver should own its own interface.
- **Use `params_schema` and `emits_schema` instead of `input` and `output`.** Rejected. `input` and `output` are the standard terms for a function or operation contract and fit the mental model of a driver as a typed black box.
- **Require every driver to declare `input` and `output`.** Rejected. Contracts are optional so existing built-in and user drivers continue to work while the ecosystem transitions.
