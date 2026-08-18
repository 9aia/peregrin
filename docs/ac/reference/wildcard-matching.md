# Wildcard matching

Event patterns in `on:` and `emits:` support wildcards.

| Pattern        | Matches                        |
| -------------- | ------------------------------ |
| `start`        | `start` only                   |
| `lint.*`       | `lint.passed`, `lint.failed`   |
| `task.**`      | `task`, `task.x`, `task.x.y.z` |
| `**.done`      | `done`, `x.done`, `x.y.done`   |
| `shard.*.done` | `shard.42.done`                |

## Match table

| Pattern         | Matches                                        | Does NOT match    |
| --------------- | ---------------------------------------------- | ----------------- |
| `shard.*.ready` | `shard.1.ready`                                | `shard.1.2.ready` |
| `shard.**.done` | `shard.done`, `shard.1.done`, `shard.1.2.done` |                   |
| `**.done`       | `done`, `x.done`, `x.y.done`                   | `x.passed`        |

## Rules

- `*` = exactly one segment.
- `**` = zero or more segments.
- Segments are dot-separated.

## Captures

Use `${event.shard}` for the first wildcard segment and `${event.match.N}`
for positional captures.

## See also

- [Events](../explanation/events.md)
- [Set up dynamic fan-out](../how-to/dynamic-fan-out.md)
