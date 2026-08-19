# human driver

Human-in-the-loop gates. Prompts a human on stdin and maps the response to
events.

## Basic usage

```yaml
- id: merge-gate
  driver: human
  on: [tests.passed]
  emits: [merge-gate.approved, merge-gate.rejected]
  with:
    prompt: "Approve merge?"
    expect: "yes"
    timeout: 60
    emit_ok: merge-gate.approved
    emit_reject: merge-gate.rejected
```

## `with:` fields

| Field         | Type   | Default                | Description                              |
| ------------- | ------ | ---------------------- | ---------------------------------------- |
| `prompt`      | string | required               | Question shown to the human              |
| `prompt_file` | string | none                   | File content prepended to `prompt`       |
| `expect`      | string | none                   | Exact response meaning "ok"              |
| `emit_ok`     | string | `emits[0]`             | Event on a matching response             |
| `emit_reject` | string | `emits[1]` or fallback | Event on a non-matching/timeout response |
| `timeout`     | int    | `0` (no limit)         | Seconds to wait for a response           |

## Response rules

- If `expect` is set, the response must match it exactly.
- If `expect` is unset, any non-empty response is treated as ok.
- If `expect` is the empty string, an empty response is ok.
- If the timeout expires, the response is treated as a rejection.

## Emitted payload

```yaml
response: the human's input (trimmed)
ok: true | false
timed_out: true | false
```

## See also

- [Drivers](../../explanation/drivers.md) — all built-in drivers
- [Safety](../../explanation/safety.md) — capability enforcement
