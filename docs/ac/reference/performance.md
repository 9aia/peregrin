# Performance

Benchmarks of the engine mechanics — throughput, concurrency, and
overhead — without invoking any AI/LLM drivers. Uses `shell` (`echo ok`)
and `python` (noop handler) drivers only.

**How to reproduce:** `python tests/benchmarks.py`

## Environment

- CPU: single machine, async Python 3.14
- Store: MemoryStore (no disk I/O)
- Cycle detection: off
- All numbers are wall-clock, single run

## Throughput

### Linear chain (sequential — one node fires the next)

| Nodes | Driver |   Time | Nodes/sec | Events/sec |
| ----: | ------ | -----: | --------: | ---------: |
|    10 | shell  | 0.016s |       622 |        746 |
|    50 | shell  | 0.142s |       353 |        367 |
|   100 | shell  | 0.283s |       353 |        360 |
|   500 | shell  | 2.398s |       209 |        209 |
|    10 | python | 0.001s |     8,187 |      9,825 |
|    50 | python | 0.007s |     6,720 |      6,989 |
|   100 | python | 0.017s |     5,912 |      6,030 |
|   500 | python | 0.367s |     1,361 |      1,367 |

**Takeaway:** the `shell` driver is bottlenecked by subprocess spawn
overhead (~2-3ms per `echo`). The `python` driver, which runs in-process,
is ~15-20x faster. For pure orchestration (no shell commands), the engine
sustains ~6,000 nodes/sec at 100 nodes.

### Parallel fan-out (all nodes on `[start]`, shell)

| Nodes |   Time | Nodes/sec | Events/sec |
| ----: | -----: | --------: | ---------: |
|    10 | 0.004s |     2,306 |      2,767 |
|    50 | 0.016s |     3,082 |      3,206 |
|   100 | 0.094s |     1,066 |      1,088 |
|   500 | 1.003s |       499 |        501 |

**Takeaway:** parallel execution amortizes subprocess overhead. At 500
nodes with default `--max-concurrency 50`, throughput is ~500 nodes/sec.

### Map-reduce fan-out (dispatch → N shards → collect)

| Shards |   Time | Nodes/sec |
| -----: | -----: | --------: |
|     10 | 0.006s |     1,993 |
|     50 | 0.037s |     1,404 |
|    100 | 0.134s |       759 |

### Self-modifying (inject N nodes at runtime)

| Injected |   Time | Nodes/sec |
| -------: | -----: | --------: |
|       10 | 0.007s |     1,728 |
|       50 | 0.025s |     2,085 |
|      100 | 0.111s |       918 |

**Takeaway:** runtime injection adds negligible overhead. 100 injected
nodes complete in 111ms.

## Concurrency: `--max-concurrency` effect

500 parallel shell nodes, varying `--max-concurrency`:

| max-concurrency |   Time | Nodes/sec |
| --------------: | -----: | --------: |
|               1 | 26.19s |        19 |
|               5 |  5.84s |        86 |
|              10 |  3.24s |       154 |
|              50 |  1.00s |       498 |
|             100 |  0.71s |       703 |
|             200 |  0.59s |       854 |

**Takeaway:** `--max-concurrency` is the primary throughput knob for
shell-heavy workflows. The default of 50 is a reasonable balance.

## Event log size

100-node linear shell workflow:

| Metric             | Value  |
| ------------------ | ------ |
| Events             | 102    |
| Total bytes (JSON) | 15,782 |
| Avg bytes/event    | 155    |

**Takeaway:** ~155 bytes/event in JSON. A 1,000-node workflow produces
~155KB. A 10,000-node workflow ~1.5MB.

## Practical guidance

- **Shell-heavy workflows:** bottlenecked by subprocess spawn. Use
  `--max-concurrency 100+` for parallel shell nodes.
- **Python-handler workflows:** ~6,000 nodes/sec at 100 nodes. The
  engine is not the bottleneck — your handler logic is.
- **Agent-driver workflows:** the engine overhead is negligible compared
  to LLM latency. The LLM provider's rate limit is the bottleneck.
- **Event log:** ~155 bytes/event. Plan disk for ~1MB per 1,000 nodes.
  Use `--no-store` for ephemeral runs.
- **Injection:** runtime node injection adds negligible overhead. Do not
  hesitate to use `driver: runner` for dynamic fan-out.
