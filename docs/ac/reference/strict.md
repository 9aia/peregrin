# Workflow strict mode

`strict` is a boolean field on the `Workflow` model. The default is `false`, which keeps the permissive behaviour that is useful for exploratory and shell-script workflows.

When `strict: true`, the engine applies a strict policy set that fails fast on missing declarations, unresolved placeholders, and missing secrets.

## Enabling strict mode

```yaml
name: strict-example
strict: true

nodes:
  - id: hello
    driver: shell
    on: [start]
    emits: [hello.done]
    params:
      cmd: "echo 'hello'"
```

## Strict behaviours

| Area                        | Permissive (`strict: false`)                                                 | Strict (`strict: true`)                                                                                 |
| --------------------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Event schemas               | Missing schemas are allowed.                                                 | Every event listed in `events` must have a schema. Payloads are validated before the event is accepted. |
| `emits`                     | A driver can return any event.                                               | Every node must declare an explicit `emits` list. Undeclared emitted events are rejected.               |
| `on` / `emits`              | Missing `on` or `emits` is tolerated.                                        | A node without `on` or `emits` is an error.                                                             |
| Placeholders                | Unresolved `${...}` expressions are left as-is or rendered as empty strings. | Any unresolved placeholder in a `params` value becomes a `TemplateError`.                               |
| Secrets                     | Missing secrets return an empty string.                                      | Referencing a missing secret fails the node.                                                            |
| Driver / handler references | Unknown drivers may be tolerated.                                            | Referencing an unregistered driver fails immediately.                                                   |
| Capabilities                | Capabilities can be inferred.                                                | Capabilities and resources must be explicitly declared.                                                 |

## Overriding per node

Individual nodes can still override the strictness of specific behaviours with explicit fields, but the workflow-level `strict` flag determines the default.

## When to use

- Use `strict: true` for production workflows where a typo or a missing value should not silently succeed.
- Use `strict: false` for early prototyping and shell-script integration.

## See also

- [Workflow schema](features.md)
- [Safety](../explanation/safety.md)
