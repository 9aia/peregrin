# Event envelope

Every unit the bus carries, nodes react to, and the log records is a
flat JSON object — the `Event` in `peregrin-core`. This is a P0
contract: later substrate work (ingress drivers, durable stores,
remote transport) builds against exactly this shape.

## Fields

| Field | Type | Meaning |
| --- | --- | --- |
| `type` | string | Routing key — dot-separated segments (`lint.passed`). Serialized as `type`; `kind` is accepted as an alias on decode. |
| `payload` | any JSON | Event data, opaque to the bus. Defaults to `{}` when absent. Validated per type by `events:`/`payload_schemas:` and driver `output` contracts. |
| `source` | string | Emitting node id, or `__runner__` for engine-synthetic events. Replay (`seed`) counts prior fires per `source`; runner-sourced events don't count. |
| `id` | string | `uuid v4` truncated to 12 chars. Observational only — defaulted on decode, never a routing input. |
| `ts` | integer | Emission time, epoch milliseconds. Observational; defaulted on decode. |

Rules:

- **The envelope is closed.** These five fields are the entire shape.
  Run-scoped routing data rides in `payload`, never at top level —
  there is no `state_key` field (v1 never had one either; scoped
  state keys are produced from `node.state.key` and surface inside
  payloads, e.g. `node.skipped`'s `{node, state_key}`).
- **CI/git metadata** (`repo`, `ref`, `sha`, `clone_url`, `actor`)
  goes inside `payload` — conventionally under a reserved `origin`
  block when a pipeline wants a standard slot. Never new top-level
  fields.
- **Nothing secret, ever.** Secrets reach nodes through the run's
  secrets record (`ctx.secrets`), the `capabilities.secrets`
  allow-list, and `${secret.*}` template resolution — see
  [Scope resolution](scope-resolution.md). The envelope carries no
  secret material so event logs stay shareable.

## Engine-synthetic events

Emitted with `source: "__runner__"` (or the affected node id for
`node.*` types). Synthetic types are exempt from payload-schema
validation.

| Type | When | Payload |
| --- | --- | --- |
| `start` | Run begins (when no `replay_events` are seeded) | `{}` |
| `workflow.done` | Pending and ready queues drained cleanly | `{}` |
| `workflow.cancel` | A cancellation request event | `{reason}` |
| `workflow.cancelled` | Run ended via cancel or `stop_on` | `{reason}` |
| `workflow.failed` | A node failure marked the run failed | `{node, error}` |
| `workflow.cycle_detected` | Reserved — cycle detection is not ported (see [Loop protection](../explanation/loop-protection.md)); no emitter today | — |
| `node.failed` | Driver error, validation failure, capability violation, retry exhausted | `{error, node}` |
| `node.exhausted` | `max_fires` cap reached (once per node) | `{node, maxFires}` |
| `node.skipped` | `state.skip_if` evaluated truthy | `{node, state_key}` |
| `event.invalid` | Payload failed schema validation | `{eventType, errors, payload}` |
| `capability.violation` | Merged capability policy denied the fire | `{node, violations}` |
| `circuit.open` | Breaker tripped (type is `circuit_breaker.on_trip`-configurable) | `{node, failures}` |
| `<node-id>.tick` | Poll-node self-trigger (auto-declared in `on`/`emits`) | `{poll: true}` |
| `<node-id>.fire` | `schedule` self-retrigger — arms the next cron occurrence (auto-declared in `on`/`emits`) | `{schedule: true}` |
| `<node-id>.scheduled` | `schedule` armed a deferred emission (auto-declared in `emits`) | `{cron, timezone, next_fire}` |
| `<node-id>.delayed` | `delay` armed a deferred emission (**not** auto-declared — strict campaigns list it in `emits`) | `{duration, duration_ms, until}` |

## Emission contract — three layers

Events reach the graph through three distinct mechanisms; only the
first is engine-checked.

**1. `emits:` declaration (engine).** The node's advertised output
types. Under `strict: true`, preflight requires every node to declare
non-empty `on:` and `emits:`, and at fire time **every event a node
produces must literally appear in `emits`** — the check is exact
match, not glob. Exempt engine types: `node.failed`, `circuit.open`,
`node.skipped` (and `event.invalid`, which is produced at publish
time, not fire time). Poll nodes get `<id>.tick` auto-appended to
`emits` at load; `schedule` nodes get `<id>.fire` + `<id>.scheduled`.

> Open question — **I-012**: exact-match `emits` makes wildcard/shard
> emit types undeclarable under strict (a node emitting
> `shard.42.done` cannot satisfy `emits: [shard.*.done]`). Glob-match
> `emits` is the likely ruling for map/shard patterns; until decided,
> strict campaigns with dynamic emit types must enumerate them.

**2. `params.emit` (driver convention).** A per-driver map of return
code → event type: `emit["<rc>"]` first, then the `emit.default`
arm. `driver: shell` uses it to turn `rc` into domain events
(`cmd.done` on `rc == 0`, `node.failed` otherwise). The engine never
interprets `params.emit` — it is an ordinary resolved param.

**3. `ctx.emit()` + `forward:` (side channels).** A driver may push
extra events onto the run's pending queue via `ctx.emit`; they are
published after that node's own returned events. Separately,
`forward:` re-emits each trigger event: for every trigger × every
`forward` pattern, a new event is produced with the trigger's payload
and `source` = this node. `*`/`**` segments inside the forward pattern
are replaced by the wildcard captures (`match.1`, `match.2`, …) of the
first `on` pattern that matched the trigger; a capture slot with no
match stays literal `*`.

Two more driver side channels shape the graph: `ctx.inject(specs)`
adds nodes mid-run (the `runner` driver reads them from the trigger's
`payload.nodes` / `payload.workflow.nodes`), and
`ctx.request_shutdown()` cancels the run after the current batch.

Outside the driver model, the **ingress injection channel**
(`peregrin_engine::ingress()`) lets external producers — webhook
listeners, WAL pollers, the supersede queue's cancel signal — push
`Event`s into a live run without `&mut` access to the bus: the
`Injector` half is `Send + Sync` and cloneable, the runner drains the
`Ingress` half into `pending` once per loop iteration, so injected
events serialize after everything already pending. See
[Queue & supersede](queue-supersede.md).
