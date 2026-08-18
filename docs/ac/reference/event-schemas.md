# Event schemas

You can declare event schemas to enforce payload contracts at runtime.
The runner validates emitted events using `jsonschema`. Validation failures
produce a WARNING log. The event is still published — validation is advisory
by default.

## Shorthand: `fields`

```yaml
events:
  - type: pr.approved
    fields: { pr_number: integer, comments: string }
  - type: pr.changes_requested
    fields: { pr_number: integer, comments: string, detail: string? }
```

Supported types: `string`, `integer`, `number`, `boolean`, `array`, `object`, `null`.
Append `?` to mark a field optional (for example `detail: string?`).

## Full JSON Schema: `schema`

```yaml
events:
  - type: batch.done
    schema:
      type: object
      properties:
        items:
          type: array
          items: { type: string }
          minItems: 1
        total:
          type: integer
          minimum: 0
      required: [items, total]
```

When `schema` is present, `fields` is ignored. The `schema` dict is a
standard [JSON Schema](https://json-schema.org/) object, validated with
`jsonschema.Draft7Validator`.

## Runtime behaviour

- Events with a declared schema are validated when published.
- Synthetic events (`start`, `workflow.done`, `workflow.cancel`, etc.)
  are exempt.
- Events without a declared schema pass through unchecked.
- Validation failures log a WARNING. The event is still published.
- `--strict` mode also emits `node.failed` and drops the invalid event.

## Reaction to invalid events

```yaml
nodes:
  - id: alert-on-bad-payload
    on: [event.invalid]
    driver: shell
    with:
      cmd: "echo 'Bad payload on ${event.payload.event_type}: ${event.payload.errors}'"
```

## File-based schemas

`*.event.json` files in `.ac/` or alongside the workflow are
auto-discovered. The filename (without extension) is the event type.

## See also

- [Events](../explanation/events.md)
- [Compose from fragments](../how-to/compose-from-fragments.md)
