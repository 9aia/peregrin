# Full feature list

## Drivers

- **shell** — run shell commands
- **python** — call Python handlers
- **runner** — inject new nodes at runtime (self-modification)
- **http** — make HTTP/API calls
- **webhook** — HTTP server that emits events from POSTs, auth, health check, localhost bind by default
- **stdin** — read lines from stdin, emit one event per line, JSON parsing
- **cli** — poll a command on an interval, emit one event per output line, self-trigger or interval mode
- **agent** — invoke external AI coding agents (devin, cline, claude-code, codex, custom) with `permission_mode`, `trust`, `prompt`/`prompt_file`, `timeout` as declarative `with:` fields. Cross-agent fallback (`runners: [devin, cline]`) with rate-limit detection, exponential backoff + jitter, and a shared per-provider cooldown so concurrent nodes don't dogpile a throttled provider
- **human** — human-in-the-loop gates with `prompt`, `expect`, `timeout`

## Flow control

- **Wildcards:** `shard.*.ready`, `**.done` for dynamic fan-out
- **Fan-in:** `mode: all` waits for every `on` pattern
- **Forward:** re-emit events without a shell echo bridge node
- **Fan-in sugar:** `fan_in:` declarative fan-in with `count` or `count_from`
- **Poll sugar:** `poll:` self-trigger pattern in one field
- **Stop on:** `stop_on:` auto-cancel without a cancel node

## Loop protection

- **`max_fires`** — cap firings per node
- **`until`** — condition to stop firing
- **`retire_on`** — event that retires a node
- **Cycle detection** — graph-level guard
- **Timeout** — workflow-level guard

## Failure events

- `node.failed` + `workflow.failed` emitted on handler exceptions — subscribe to react

## Resilience

- **Retry** with backoff
- **Dead-letter queue** (persistent by default)
- **Circuit breaker**

## Persistence

- Crash-recoverable by default (JsonStore at `.ac/<name>.jsonl`)
- `--replay` resumes from crash
- `--resume` retries from failure (prunes failure events, resets fire counts, preserves worktree state)
- `--no-store` opts out

## Observability

- `--otel` enables OpenTelemetry tracing
- `--otel-endpoint` exports to OTLP collectors (Jaeger, Tempo, Honeycomb, Datadog)
- `--log-format json` for ELK, Datadog, Loki
- `--metrics 9090` exposes Prometheus `/metrics` and `/health` endpoints

## Composition

- **Includes anywhere:** `include:` (merge) and `${include(path)}` (inject) in any YAML position
- **Named snippets:** `snippet: name` (merge) and `${snippet(name)}` (inject) — reusable fragments by name, with file refs and inheritance
- **Python imports:** `import:` in YAML — import handler modules from file paths or dotted module paths, with `as:` aliasing
- **File router:** auto-discover `*.node.yml`, `*.snippet.yml`, `*.event.json`, `*.handler.py`, `*.config.yml`, `*.secret.yml`, `.env` files by suffix — colocate by domain, walked recursively in `.ac/` first, then alongside
- **Multi-file:** split workflows across files with recursive includes

## Validation and types

- **Payload validation:** `*.event.json` — JSON Schema per event type, validated at runtime. Failures emit `event.invalid`. `--strict` makes failures also emit `node.failed` and drop the invalid event
- **Type generation:** `ac typegen workflow.yaml` — generates JSON Schema with enums for event types, snippet names, handler names, and driver types for editor autocomplete

## Streaming and dry-run

- **Streaming:** `with.stream: true` on shell/http — emit one event per line as output arrives
- **Dry-run:** `--dry-run` simulates without side effects (shell/http/webhook print instead of execute)

## Placeholders and state

- **Payload placeholders:** `${event.payload.x}`, `${secret.API_KEY}`, `${config.region}`, `${state.x}`, `${event.shard}`, `${node.fires}`
- **Shared state:** `ctx.state` — persisted key/value store (default `.ac/<name>.state.json`), restored on `--replay`
- **Wildcard captures:** `${event.shard}` and `${event.match.N}`
- **Fire counter:** `${node.fires}` / `ctx.fires` — 1-based attempt number
- **Idempotency keys:** `ctx.idempotency_key` — unique per node firing

## Secrets

- `--secrets` (JSON), `--env` (.env file), auto-loads `.env` from workflow dir, process env vars
- `AC_PUBLIC_*` convention for config vs secrets
- Automatic redaction in CLI output, export logs, and dry-run

## Operational

- **Graceful shutdown:** SIGINT/SIGTERM handler — awaits in-flight tasks, cleans up webhook servers
- **Concurrency control:** `--max-concurrency N` limits parallel node executions (default: 50)
- **Reusable lib:** `packages/ac-examples/examples/lib/` — canonical, copyable node YAMLs

## Integration library

- `litellm` — unified LLM calls (OpenAI, Anthropic, Google, 100+ providers)
- `apprise` — unified notifications (Slack, Telegram, Discord, 75+ services)
