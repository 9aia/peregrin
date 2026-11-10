# Drivers

Drivers execute node logic. A node declares `driver: <name>`, and the
runner looks the name up in its registry at fire time. The driver
receives the node, the trigger events, and the execution context —
it runs the action and returns the events to publish.

```rust
pub trait Driver: Send + Sync {
    fn name(&self) -> &str;
    fn input(&self) -> Option<EventSchema> { None }
    fn output(&self) -> &[EventSchema] { &[] }
    fn invoke(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError>;
}
```

`input` is the contract `params` validates against (when the node
doesn't declare its own `input:`); `output` holds per-emitted-type
payload contracts. The runner validates resolved params and emitted
payloads against them before events reach the bus.

## The unified registry

`peregrin_engine::Registry` is one flat map of names to drivers —
there is no separate kind of registry for triggers, actions, or
sinks (ADR-0082 §4). An **ingress driver** is just a driver that
emits asynchronously for the life of the run instead of returning
its events; a **sink** is just a driver whose node fires on terminal
campaign events and publishes the verdict out.

Resolution is exact match first, then `:`-prefix — `driver:
agent:claude` resolves the driver registered under `agent`, and the
suffix is the driver's own to interpret.

```rust
let registry = Registry::default()
    .with("shell", Arc::new(ShellDriver::new()))
    .with("my-driver", Arc::new(MyDriver));
```

A stock node resolves (`peregrin_node::campaign::registry`):

| Name | Kind | What it does |
| --- | --- | --- |
| `shell` | action | `params.cmd` under the system shell; `params.emit` maps `rc` → event type (30 s default, 600 s cap). |
| `http` | action | `params.url`/`method`/`headers`/`body` fetch; 2xx → `http.done`, else `http.failed`. `params.notify` hits the apprise-style notify path. |
| `agent` | action | `params.runner: devin` (default) spawns the agent CLI; `custom` runs `params.binary` + prompt on stdin. `params.model` switches to a LiteLLM chat-completions call. |
| `human` | action | `params.prompt` to stderr, line from stdin; `params.expect`/`emit_ok`/`emit_reject`. |
| `git` | action | Materializes a detached worktree at `params.sha` from `params.url`, runs an `inner` driver inside it, cleans up. `token_env`/`ssh_env` bind credential *names*. |
| `github-status` | sink | `POST /repos/{repo}/statuses/{sha}` — the commit-status verdict; `state` auto-maps off the trigger's `.done`/`.failed` suffix. |
| `delay` | temporal | Arms `emits[0]` for `now + params.duration` (`5m`, `1h30m`, `500ms`) on the engine's timer heap — the wait doesn't block the run (ADR-0018). Returns `{id}.delayed`; the landed emit carries `{duration, until}` — or the trigger's payload verbatim under `params.passthrough` (one deferred emit per trigger). |
| `subworkflow` | action | Runs another campaign YAML (`params.path`, resolved like `include:`) as an isolated sub-run — own bus/log/state, no ingress. `params.inputs` merge over the sub-campaign's `config`; `params.outputs` maps sub event types → parent types (ADR-0022). |
| `council` | action | One node = a voting round over `params.members` (`agent:<runner>`, `human`, `driver:<name>`, `{driver, params, label}`). Tally rules `unanimous`/`majority`/`supermajority`/`any`, `tie_breaker`, `timeout`; emits carry `{votes, counts, winner, quorum}` (ADR-0004). |
| `schedule` | temporal | Emits `emits[0]` at each `params.cron` occurrence in `params.timezone` (IANA, default UTC; 5- or 6-field cron). Recurs through a synthetic `{id}.fire` re-arm; `max_fires` bounds emissions (`1` = one-shot, `unlimited` = default). |
| `noop` | builtin | Emits `node.done`; tests and scaffolding. |
| `runner` | builtin | Injects node specs from the trigger's `payload.nodes` — the self-modifying primitive. |

Two ingress drivers exist as crates — `github-webhook` (HMAC-verified
push + `pull_request` listener; PR actions filter via
`params.pull_request_actions`, default `opened,synchronize,reopened`)
and `sturn-poll` (git-ref WAL poller) — and compose
the same way once the node's per-run wiring hands them the run's
`Injector`. The v1 `python`/`cli`/`stdin`/generic `webhook`/
`streaming` drivers and the litellm/apprise plugin drivers are not
ported.

## What a driver gets — `ExecutionContext`

Beyond the node and triggers, `ctx` carries the shared run state:
`config`, the fire's resolved `params`, `secrets` (the run's secrets
record), `state` (scoped when `state.key` is set), `capabilities`,
`fires`, `idempotency_key`, `traceparent`, `dry_run`, and the strict
`missing_secrets` audit ledger.

Four side channels shape the graph:

- `ctx.emit(event)` — queue extra events; they publish after the
  node's own returned events.
- `ctx.inject(specs)` — add nodes mid-run (the `runner` driver reads
  them from the trigger payload).
- `ctx.schedule_at(event, at)` / `schedule_in(event, dur)` — defer an
  emission onto the runner's timer heap; the event publishes when the
  deadline lands and holds the run open until then (ADR-0018).
- `ctx.run_sub(campaign, opts)` — run a nested campaign on a fresh,
  isolated `Runner` (own bus/log/state; the `subworkflow` driver's
  channel, ADR-0022).
- `ctx.request_shutdown()` — cancel the run after the current batch.

Drivers also template-resolve their own params a second time inside
`invoke` (against the bound trigger) — the `${event.*}`/`${config.*}`
forms a param can carry are specified in
[Scope resolution](../reference/scope-resolution.md).

## Secret names, never secret values

Drivers that need credentials take a `*_env` param naming a *secret
key* — `token_env`, `secret_env`, `ssh_env` — resolved through the
`capabilities.secrets` gate and `ctx.get_secret`. Values never sit in
`params` or config; the run's secrets record is the only source. See
[Safety](safety.md).

## See also

- [Event envelope](../reference/event-envelope.md) — the three
  emission layers (`emits:`, `params.emit`, `ctx.emit`/`forward:`)
- [Queue & supersede](../reference/queue-supersede.md) — the ingress
  injection channel ingress drivers push through
- [Campaign schema](../reference/campaign-schema.md) — node `input:`/
  `output:`/`capabilities:` fields
