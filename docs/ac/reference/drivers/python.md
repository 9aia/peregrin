# python driver

Calls a Python handler function. The handler receives `(triggers, ctx, node)`
and returns a list of events.

## Register by name

```python
from agent_campaign.drivers.python import register

register("review_pr", review_pr)
```

```yaml
- id: review
  driver: python
  handler: review_pr
  on: [pr.opened]
  emits: [pr.approved, pr.changes_requested]
```

## Use `module:func` notation

```yaml
- id: review
  driver: python
  handler: myapp:review_pr
  on: [pr.opened]
  emits: [pr.approved, pr.changes_requested]
```

## Common `with:` fields

| Field     | Type   | Description                           |
| --------- | ------ | ------------------------------------- |
| `handler` | string | Registered name or `module:func` path |

The `ctx` object is documented in [Handler context](../handler-context.md).

## See also

- [Handler context](../handler-context.md)
- [Handler registration](../handler-registration.md)
- [Write a Python handler](../../how-to/custom-drivers.md)
