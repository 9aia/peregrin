# Resource management

Resource exhaustion is different from exceptions: heavy handlers spawn OS
processes that can thrash the machine. The engine adds backpressure.

## Concurrency

```bash
ac run wf.yaml --max-concurrency 4 \
  --max-cpu-percent 80 --max-mem-percent 85 --min-free-mem-mb 512
```

Requires `psutil` (`pip install -e '.[resource]'`). Without it, the gate
falls back to count-only.

## Per-node weight

```yaml
- id: vitest
  driver: shell
  on: [start]
  emits: [test.passed, test.failed]
  resources: { weight: 4 }
  with: { cmd: "vitest run" }
```

`weight` defaults to 1 and is capped to `max-concurrency`.

## Global load circuit breaker

```bash
ac run wf.yaml \
  --load-breaker-cpu 90 --load-breaker-mem 90 \
  --load-breaker-sustained 10 --load-breaker-cooldown 30
```

Trips after sustained overload and emits `system.overloaded`.

## Per-process OS limits

```yaml
- id: agent
  driver: agent
  on: [start]
  emits: [agent.done]
  with:
    runner: devin
    prompt: "..."
    mem_limit_mb: 4096
    oom_score_adj: 500
```

- `mem_limit_mb` hard-caps the child address space.
- `oom_score_adj` (higher = more likely killed) protects the parent.

## CLI flags

| Flag                       | Description                               |
| -------------------------- | ----------------------------------------- |
| `--max-concurrency`        | Max parallel node executions (default 50) |
| `--max-cpu-percent`        | Defer until CPU is below this             |
| `--max-mem-percent`        | Defer until memory is below this          |
| `--min-free-mem-mb`        | Defer until free memory is above this     |
| `--load-breaker-cpu`       | CPU threshold for global load breaker     |
| `--load-breaker-mem`       | Memory threshold for global load breaker  |
| `--load-breaker-sustained` | Consecutive samples before tripping       |
| `--load-breaker-cooldown`  | Seconds before half-open                  |

## See also

- [Resilience](../explanation/resilience.md)
- [Run a workflow](../tutorials/run-a-workflow.md)
