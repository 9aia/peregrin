# Handler registration

Python handlers can be used by name or by `module:func` path.

## Register by name

```python
from agent_campaign.drivers.python import register
from agent_campaign.model import Event

def my_handler(triggers, ctx, node):
    return [Event(type="done", payload={"result": "ok"}, source=node.id)]

register("my_handler", my_handler)
```

```yaml
- id: custom
  driver: python
  handler: my_handler
  on: [start]
  emits: [done]
```

## `module:func` notation

No registration needed:

```yaml
- id: custom
  driver: python
  handler: myapp.code_review:review_pr
  on: [pr.opened]
  emits: [pr.approved, pr.changes_requested]
```

The `myapp.code_review` module is imported and `review_pr` is called.

## Import modules in YAML

```yaml
import: [lib/litellm/handlers.py]

nodes:
  - id: review
    driver: python
    handler: llm:complete
    on: [pr.opened]
    emits: [review.done]
```

With an `as:` alias:

```yaml
import:
  - file: lib/litellm/handlers.py
    as: llm
```

Dotted module paths (for installed packages):

```yaml
import: [myapp.handlers]
```

## See also

- [Handlers](../explanation/handlers.md)
- [python driver](drivers/python.md)
