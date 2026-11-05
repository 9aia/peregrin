# Campaign schema

The full YAML → `Campaign` contract, ported from v1 `model.ts`. The
typed model lives in `peregrin-core` (`Campaign::from_dict`,
`Node::from_dict`); expansion passes (`include`, `snippet`,
discovery) run first in `peregrin-loader-yaml`. `from_dict` is total
for nodes — bad scalars coerce or drop — but `nodes:`/`events:` that
aren't arrays are a hard error.

## Campaign fields

| Field | Type | Meaning |
| --- | --- | --- |
| `name` | string | Campaign name. |
| `nodes` | `[Node]` | The graph — see node fields below. |
| `events` | `[EventSchema]` | Declared event contracts; under `strict` every entry must compile a payload schema. |
| `functions` | map or `[FunctionDefinition]` | YAML-declared template functions. Map form injects the key as `name`. Fields: `driver`, `params`, `capabilities`, `requires_capability`, `deterministic`, `returns`. |
| `config` | map | Campaign config — `${config.*}` in templates; also `fan_in.count_from` source. |
| `include` | `[path]` | File composition (alias `includes`). |
| `import` | `[path \| {file, as}]` | Python modules for python-driver handlers. |
| `stop_on` | `[type]` | Exact-match run terminators. |
| `payload_schemas` | `{type: schema}` | Per-event-type JSON schemas; `*.event.json` discoveries fold in here. |
| `capabilities` | map | Workflow capability policy (secrets, agents, urls, shell/http/python/webhook allow-deny, `default_policy`). |
| `strict` | bool | Strict validation mode (ADR-0001); JS-truthy parse. |
| `state` | map | Campaign-level state seed for `${state.*}`. |
| `annotations` | map | Free-form metadata (ADR-0009). |
| `snippets` | `{name: body}` | **Load-time only** — consumed by the expansion pass; never reaches the typed model. |

## Node fields

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `id` | string | `""` | Unique within the campaign. |
| `driver` | string | `""` | Registry name (`shell`, `agent`, `runner`, `noop`, …). Empty driver = pure event node. |
| `handler` | string | `""` | Handler reference within the driver. |
| `on` | `[pattern]` | `[]` | Glob subscription — see [`on:` grammar](on-grammar.md). |
| `emits` | `[type]` | `[]` | Declared output types — strict-checked **exact**. |
| `mode` | `any` \| `all` | `any` | Trigger semantics. |
| `params` | map | `{}` | Driver params — template-resolved at fire time. |
| `max_fires` | int \| `"unlimited"` | `1` | Fire cap. `0`/negative/non-integer falls back to the default (v1 quirk, kept — strict rejection is an ADR-0082 *fix* item). |
| `until` | `[type]` | `[]` | Exact-match stop-listening list. |
| `retire_on` | `[type]` | `[]` | Exact-match retire list — removes the node from the live graph. |
| `retry` | map | `{}` | `max_attempts` (default 1), `delay` secs (default 1), `backoff`: `fixed` \| `linear` \| `exponential`. Exhausted → `node.failed` + DLQ enqueue. |
| `circuit_breaker` | map | none | Present → breaker armed. `threshold` (default 5), `reset_after` secs (default 60), `on_trip` event type (default `circuit.open`). Success resets. |
| `poll` | map \| truthy | none | Presence flips to poll mode — see normalization below. |
| `fan_in` | map | none | Count join over `on[0]`: `count`, `count_from` (config path), `emit` override. |
| `forward` | `[pattern]` | `[]` | Re-emit each trigger with capture substitution — see [Event envelope](event-envelope.md). |
| `resources` | map | `{}` | `weight` (min 1) against the run's concurrency gate (cap 50 — inert under the sequential runner, kept for parity). |
| `capabilities` | map | `{}` | Node capability policy — merged over the workflow's (see below). `merge: true` unions list keys. |
| `state` | map | `{}` | `key` — template resolving to a scoped-store key; `skip_if` — template, truthy (`true`/`yes`/`1`/`ok`, trimmed, case-insensitive) skips the fire with `node.skipped`. |
| `annotations` | map | `{}` | Free-form metadata. |
| `input` | EventSchema (no `type`) | none | Params contract — overrides `driver.input`. |
| `output` | `{type: EventSchema}` | `{}` | Per-emitted-type payload contracts; node entries override driver `output`. |

## `EventSchema`

`{type, fields, schema, annotations}` — used by `events:`, node
`input:`/`output:`, and driver contracts.

- `fields` shorthand: `name: <json-type>[?]` where `?` marks optional;
  unknown type names compile to `string`.
- `schema` is a full JSON-schema dict and **wins over `fields`** when
  non-empty.
- Compiled schema checks `required` properties and per-property
  primitive `type` (`string`/`integer`/`number`/`boolean`/`array`/
  `object`/`null`); non-object payloads fail `(root)`.

## Load-time normalization

Applied by `Node::from_dict` / the loader expansion, in order:

1. **Discovery** — `.peregrin/` then `.handl/` then the base dir
   (recursive, dot-dirs skipped; the named roots win name dedupes in
   scan order — `.peregrin` is the rebrand name, `.handl` stays for
   v1 compatibility): `*.node.yml`, `*.snippet.yml`,
   `*.event.json`, `*.config.yml`, `*.fn.yml` fold into `nodes`,
   `snippets`, `payload_schemas`, `config`, `functions`.
   `*.handler.py`, `*.secret.yml`, `.env` are discovered for other
   consumers.
2. **`include:`** — a dict-level key deep-merges file contents *under*
   local keys; the top-level `include:` list does the same per path.
   `${include(path)}` inside strings splices the parsed value (typed
   when it's the whole string). Circular includes error by path.
3. **`snippets:`** — `snippet: <parent>` inheritance inside the table
   resolves to a fixpoint (local wins); `snippet: <name>` in any dict
   merges the body under local keys; `${snippet(name)}` interpolates
   like `${include()}`. Cycles and non-dict bodies error.
4. **Deep merge** — dicts merge per-key local-wins, arrays concatenate
   local-first, scalars take local.
5. **`with:` rejection** — any node `with:` key is a hard load error
   (renamed `params`, ADR-0025).
6. **Poll normalization** — a truthy `poll:` appends `<id>.tick` to
   `on` and `emits`, merges `poll.until` into `until`, and backfills
   `params.interval` from `poll.interval`.
7. **Self-trigger default** — `driver: cli`/`polling` with
   `params.mode != "interval"` defaults `max_fires: 100` (others: 1).

Coercion quirks kept from v1: scalar strings in list positions
iterate into characters (`on: start` → `["s","t","a","r","t"]`) —
always write lists; `strict`, `requires_capability`, `deterministic`
parse JS-truthy.

## Capability merge

`capabilities` merges `{...workflow, ...node}` for the template ctx
and via `CapabilityChecker.merge` for driver-param checks: node keys
override; dict-list keys (`shell`, `http`, `python`, `webhook`) merge
their `allow`/`deny`/`ports`/`hosts`/`paths` lists; top-level list
keys (`secrets`, `agents`, `urls`) replace — or union when the node
sets `merge: true`. `default_policy` defaults to `deny` under
`strict`, `allow` otherwise.
