# python driver

The `python` driver spawns the `python` interpreter. It is a shell-out driver, not a Python SDK.

## Run a Python script

```yaml
- id: analyse
  driver: python
  on: [data.ready]
  emits: [analyse.done, analyse.failed]
  params:
    script: scripts/analyse.py
    args: ["--input", "${event.payload.path}"]
    timeout: 60000
```

`python` is invoked with the script path and any `args`.

## Call a function in a module

```yaml
- id: compute
  driver: python
  on: [start]
  emits: [compute.done, compute.failed]
  params:
    module: myapp.math
    function: compute_total
```

The driver runs:

```bash
python -c "from myapp.math import compute_total; print(compute_total())"
```

## Common `params` fields

| Field      | Type   | Description                             |
| ---------- | ------ | --------------------------------------- |
| `script`   | string | Path to a Python script to execute      |
| `module`   | string | Module to import (requires `function`)  |
| `function` | string | Function to call (requires `module`)    |
| `args`     | array  | Command-line arguments for `script`     |
| `timeout`  | number | Timeout in milliseconds (default 30000) |

## Exit events

- `python.done` — exit code `0`. Payload includes `stdout` and `rc`.
- `python.failed` — non-zero exit code or runtime error. Payload includes `stdout`, `stderr`, `rc`, and `error`.

## See also

- [Execution context](../handler-context.md)
- [Register a custom driver](../handler-registration.md)
- [Custom drivers and handlers](../../how-to/custom-drivers.md)
