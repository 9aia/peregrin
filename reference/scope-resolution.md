# Scope resolution

`${event.*}`, `${config.*}`, `${params.*}`, `${state.*}`,
`${node.*}`, `${secret.*}` — the scope-resolution pass. This contract
is net-new: v1 never had it (its tests assert `${config.region}`
passes through literally), and the ported `libraries/` artifacts are
dead without it. It specifies *what* the pass resolves and *where* it
sits so any later phase can implement it without re-deriving
semantics.

## Position in the pipeline

`resolve_template` (peregrin-template) resolves a string in four
stages. The scope pass is stage 3:

1. **Default normalization** (`resolve_defaults`) — rewrites
   `${x :- d}` to canonical `${x:-d}`, recursing into defaults.
   Text-only; resolves nothing.
2. **Function resolution** (`render_template`) — scans `${`…`}` (up
   to the first `}`, non-empty interior); `${name(args)}` /
   `${name.method(args)}` with a known *and granted* function
   resolves; every other placeholder is emitted verbatim. Function
   calls are fully consumed here — the scope pass never sees them.
3. **Scope resolution** *(this spec)* — re-scans the rendered text
   and resolves scope-namespaced placeholders. Misses stay verbatim.
4. **Strict audit** — under `strict: true`, any remaining `${...}`
   that isn't a granted function call fails with
   `unresolved placeholders: …`; a non-empty missing-secrets ledger
   fails with `missing secrets: …`.

Stage 3 must run *after* function resolution (functions win ambiguous
syntax) and *before* the strict audit (a failed scope lookup must be
visible to the audit). In lenient mode the audit doesn't run and
every unresolved placeholder passes through verbatim — the existing
doctrine, unchanged.

## Grammar

```
placeholder  := "${" scope-expr "}"
scope-expr   := ns ( "." segment )+ ( ":-" default )?
ns           := "event" | "config" | "params" | "state" | "node" | "secret"
```

- `segment` is any non-empty run between `.` separators — dotted
  paths walk nested objects one key per segment. A path that needs a
  key containing `.` is unreachable by design (name keys to avoid it).
- A bare `${ns}` with no path is **unresolved** (no meaningful string
  form) — except `${event.match.N}`/`${event.shard}` style paths,
  which are normal segments.
- `secret` is the exception: `${secret.NAME}` treats everything up to
  `:-` or `}` as a single key, so secret names may contain dots.
- Anything that parsed as a function call in stage 2 — or uses `|`
  filters — is not a scope expression and is left alone.
- Resolved values stringify via `toStringSafe`: `null` → `""`,
  booleans/numbers as JS `String(v)`, arrays join with `,`
  (null elements → `""`), objects → `[object Object]`.

## Namespaces

| Prefix | Bound to | Forms |
| --- | --- | --- |
| `event` | `triggers[0]` of the current fire | `type`, `id`, `ts`, `source`, `payload.<path>`, `match.<N>`, `shard` |
| `config` | the campaign `config:` map | `config.<path>` |
| `params` | the node's `params:` — lazily resolved | `params.<path>` |
| `state` | the ctx state store (scoped store when `state.key` is set, else the shared run store) | `state.<key>[.<path>]` |
| `node` | the firing node | `id`, `fires` |
| `secret` | the run's secrets record, capability-gated | `secret.<name>` |

### `event`

`event` binds to the **first trigger** of the current fire —
`triggers[0]` in the vector the bus produced (for `mode: all` that's
the earliest `on` pattern's first match; for `fan_in` the oldest
matching event; after `seed`/`check_ready` the first logged match).
Multi-trigger nodes needing per-trigger data should read
`fan_in`'s payload or publish side events — `event` deliberately
means "the trigger".

- `event.type`, `event.id`, `event.ts`, `event.source` — envelope
  fields of the bound event.
- `event.payload.<path>` — walks the payload object; a non-object
  step or missing key is a miss.
- `event.match.<N>` / `event.shard` — wildcard captures from the
  **first `on` pattern that matched the bound trigger**
  (`capture_wildcards`); `shard` aliases `match.1`. No wildcard or no
  matching pattern → miss.
- When no trigger is bound (resolutions outside a fire —
  e.g. a driver re-resolving without one) every `event.*` lookup
  misses.

### `config`

`config.<path>` walks the campaign `config:` map. Missing key or
non-object step → miss.

### `params`

`params` resolves against the node's declared `params:` map with
**lazy memoization**:

- During resolution of `node.params` itself, `params.<path>`
  resolves the referenced key through the full pipeline
  (defaults → functions → scope) on first reference and memoizes it
  for the rest of that resolution pass.
- A circular reference — `a` referencing `params.a` directly or
  transitively — is a miss (the in-progress key is never visible).
- Outside params resolution (`state.key`, `state.skip_if`,
  `ctx.emit` payloads, `forward` destinations, driver-side
  re-resolves), `params` binds the **already-resolved** params
  object produced for the fire.
- Path walk: dict keys only; missing key or scalar step → miss.

### `state`

`state.<key>` looks up `<key>` (the first path segment) in the ctx's
state store — the scoped `state.key` store when the node declares
one, else the shared run store — then walks remaining segments into
the stored record. Absent key → miss. This gives nodes read access
to their own record and to shared campaign state; writes stay on
`ctx.state` in drivers.

### `node`

`node.id` (string) and `node.fires` (number — the fire count
including the current fire). Unbound when the resolve call carries
no node → miss.

### `secret`

`${secret.NAME}` resolves through the same contract as
`ctx.get_secret(NAME)`:

1. **Gate** — `capabilities.secrets` (the merged ctx capability map)
   must be present and not `false`; otherwise the placeholder is
   unresolved. (Mirrors the function gate: presence whitelists.)
2. **Match (strict only)** — under `strict`, the grant must match
   `NAME`: `true` allows all, a list is name globs, `{allow, deny}`
   applies deny-then-allow. A non-matching grant fails with
   `capability 'secrets' not granted for 'NAME'` — the same error
   surface as `${env(...)}`/`${file(...)}`/`${shell(...)}` grants.
   Non-strict skips argument matching, like every other capability.
3. **Lookup** — resolves through `ctx.resolve_secret`: the run's
   secrets record, then ambient process env, then the fallback
   stores (`secret_stores`; OS keyring — consulted only for
   grant-matched names — then `~/.config/peregrin/env`). A hit
   substitutes the value. **Miss substitutes `""`** and, under
   strict, records `NAME` in the missing-secrets ledger —
   the audit then fails with `missing secrets: NAME`.

Ruling on the miss: substitute `""`, not pass-through. It mirrors
`ctx.get_secret`/`ctx.secrets[name]` exactly — one secrets contract,
two access surfaces — and matches `${env("UNSET")}` → `""`. A
verbatim `${secret.NAME}` leaking into a resolved param would put
placeholder text into driver inputs and would be *less* auditable
than the ledger. Users wanting a fallback write
`${secret.NAME:-fallback}`.

## Misses, defaults, and strict mode

A **miss** is any failure to resolve: unknown namespace, missing key,
unbound `event`/`node`, ungated `secret`, circular `params`, a scalar
mid-path. Miss handling:

- `${ns.path}` → the placeholder is emitted **verbatim**. In lenient
  mode that's the final result (pass-through doctrine). Under strict
  the stage-4 audit flags it — the audit's resolvable-looking set
  shrinks to granted function calls and the five I/O-builtin dotted
  forms (`env`, `file`, `shell`, `random`, `uuid`, `now`); leftover
  scope-namespace placeholders now count as unresolvable.
- `${ns.path:-default}` → the lookup miss substitutes `default` and
  the substituted text is re-resolved through the **full pipeline**
  (defaults normalization → functions → scope), so
  `${config.a:-${config.b}}` chains and
  `${config.a:-${env("FALLBACK")}}` both work. The depth of chained
  substitutions is capped (8); exceeding the cap treats the
  expression as a miss. A default is only consulted on a miss —
  `${config.a:-d}` with `a` present resolves to `a`, even when `a`
  is `""` or `null` (which stringify to `""` anyway). This is the
  first feature to actually consume `:-` defaults: stage 1 only ever
  normalized their text.

## Binding summary for implementers

The runner supplies scope bindings alongside the existing template
`Context`/`Options` (`strict`, `missing_secrets`, `capabilities`,
`effects`, `node`, `functions`):

| Binding | Source at a node fire |
| --- | --- |
| `event` | `triggers[0]` + the node's `on` patterns (for captures) |
| `config` | `ctx.config` (campaign `config:`) |
| `params` | declared `node.params` during param resolution; resolved params afterwards |
| `state` | `ctx.state` — scoped when `state.key` is declared, shared otherwise |
| `node` | `node.id`, `ctx.fires` |
| `secret` | `ctx.secrets` gated by `capabilities.secrets` |

All node-fire-time resolutions — `params`, `state.key`, `state.skip_if`,
and driver-side re-resolves that receive a trigger — bind the same
`event`. Resolutions with no fire context leave `event` and `node`
unbound.
