+++
id = "PL-001"
title = "Distributed / production-ready plan"
status = "open"
priority = "medium"
+++

# Distributed / production-ready plan

A production-ready distributed version would need these steps:

1. **Pick a topology** — multi-process on one host, multi-host, or container/k8s.
2. **Distribute the event bus** — swap the in-process bus for a persistent broker (NATS, Redis Streams, Kafka, RabbitMQ) with durable queues.
3. **Centralize state** — move the `Store` to a database (Postgres, Redis, CockroachDB, etc.) or go fully event-sourced.
4. **Decouple files from CWD** — use a shared FS (NFS/EFS), object storage (S3/GCS), or per-agent worktrees with explicit artifact passing.
5. **Remote agent runners** — run `devin`/`claude-code`/etc. in worker processes or containers that pull jobs from the bus, with health checks and heartbeats.
6. **Guarantee delivery/ordering** — add idempotency keys and at-least-once delivery; for `mode: all` fan-in, use vector clocks or bounded waits.
7. **Isolate and secure** — sandboxed worker containers, secrets management, auth between bus, workers, and store.
8. **Observability** — structured logs, tracing per workflow run, metrics on node fire/exhaust, and DLQ for poison events.
9. **Harden failure handling** — distributed `node.exhausted` semantics, circuit breakers, retry budgets, and canary dry-runs.
10. **Run distributed dry-run/validate** — make `handl validate` and `handl run --dry-run` aware of the real multi-worker topology.

Start with the event bus and the store; those two decisions determine the rest.
