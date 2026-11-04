# `on:` event patterns

The grammar every `on:` entry (and every glob-matched event-type
position) uses. Matching lives in `peregrin-bus`; the semantics are
ported 1:1 from the v1 `matchPattern`/`captureWildcards`.

## Pattern grammar

Event types are dot-separated segments. A pattern is the same shape
with two wildcard segments:

| Segment | Matches | Capture |
| --- | --- | --- |
| literal | itself, exactly | — |
| `*` | exactly one segment | one segment |
| `**` | zero or more segments | all matched segments, re-joined with `.` |

Only a whole segment can be a wildcard — `fo*` is a literal segment,
not a prefix glob.

The matcher backtracks left to right: `**` is tried from zero
segments upward and the first split that lets the remainder of the
pattern match wins. Match/no-match is unaffected by that order; it
only decides captures for ambiguous patterns like `**.**` (the first
`**` takes the minimal split).

| Pattern | Matches | Does not match |
| --- | --- | --- |
| `start` | `start` | `start.x` |
| `lint.*` | `lint.passed` | `lint`, `lint.a.b` |
| `task.**` | `task`, `task.x`, `task.x.y.z` | `taskx` |
| `**.done` | `done`, `x.done`, `x.y.done` | `x.passed` |
| `shard.*.done` | `shard.42.done` | `shard.1.2.done` |

## Captures

Each wildcard captures; captures are 1-indexed in pattern order across
`*` and `**` (`**` captures join back with `.`):

- `${event.match.N}` — the Nth wildcard's captured text.
- `${event.shard}` — alias for `match.1` (the shard idiom
  `shard.*.done` → `shard` = `42`).

Captures come from the first `on` pattern that matched the trigger
event — the same source `forward:` substitution uses. See
[Scope resolution](scope-resolution.md) for the `${event.*}` forms.

## Trigger semantics — `mode:`

`on:` subscribes a node to event types; `mode` picks how the
subscription fires:

| `mode` | Fires when | `triggers` bound for the fire |
| --- | --- | --- |
| `any` (default) | each published event matches any `on` pattern | the single matching event |
| `all` | every `on` pattern has ≥1 matching event in the log | first matching event per **distinct** pattern, in `on` order |

`mode: all` is the fan-in primitive: once satisfied it fires once
(duplicate identical patterns count once). With `max_fires` it can
fire again — each fire needs all patterns satisfied by the log.

`fan_in:` is the count-based alternative: it considers only `on[0]`
and fires when the number of logged events matching that pattern
reaches `count` (or `count_from`, a `config` dotted path resolving to
an array length or number; default 1). The driver is skipped and the
node emits `fan_in.emit` (else `emits[0]`, else `<id>.done`) with
payload `{count, triggers: [{type, source, payload}, …]}`.

## Eligibility gates

Independent of `mode`, a node stops being a candidate when:

- **`max_fires`** — fire count reaches the cap; once exhausted, a
  `node.exhausted` side event is emitted the next time an event
  matches (for `mode: all`, only once all patterns are satisfied).
  `unlimited` never exhausts.
- **`until`** — any listed type has been seen; also suppresses the
  `node.exhausted` side event.
- **`retire_on`** — any listed type has been seen; the node is
  removed from the live graph entirely.

## Exact-match positions

Verified against v1: **only `on:` patterns glob-match** (plus the
`fan_in` count pattern, `forward` capture source, and the `output:`
schema lookup — exact first, then wildcard). These positions compare
literally and do **not** glob:

- `until:` — stop-listening list
- `retire_on:` — retire list
- `stop_on:` — campaign-level run terminators
- `emits:` strict check — declared-type verification

Keeping `until`/`retire_on`/`stop_on` exact is deliberate: lifecycle
lists name concrete terminator events. The `emits:` gap is tracked as
I-012 — see [Event envelope](event-envelope.md).
