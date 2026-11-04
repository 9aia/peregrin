# Queue & supersede policy

The engine executes **one** run at a time; the supersede policy
decides *which* runs happen when trigger events outpace the
executor. This is node-level policy, not engine internals — it lives
in `peregrin-queue` (pure logic, no I/O) and the node daemon drives it
around each `Runner`. The `peregrin-node` `RunSupervisor` composes the
queue with the engine's ingress channel into the sequential run loop
the daemon will own.

## `supersede:` campaign config

```yaml
supersede:
  key: "{repo}:{ref}"     # template over the trigger event payload
  mode: latest-wins       # | fifo
  cancel_running: true    # cancel the active run vs queue behind it
```

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `key` | template | `""` | `{dotted.path}` placeholders resolved against the trigger event's `payload`; events resolving to the same key are alternative triggers for the same logical run (one branch, one PR, one WAL head). Empty = a constant key, so every event targets the same run. |
| `mode` | `latest-wins` \| `fifo` | `latest-wins` | Pending-job dedupe: `latest-wins` replaces the pending job that shares the key (only the newest trigger for a target runs — the displaced job's queue *position* is inherited); `fifo` appends (every trigger runs). |
| `cancel_running` | bool | `false` | `true`: a new event marks the *active* run superseded — the queue raises a one-shot cancel signal the caller delivers to the run. `false`: new events always queue behind the active run. |

## Key template grammar

Distinct from `${...}` scope resolution — the `{path}` grammar is a
small payload-only interpolation:

- `{repo}`, `{origin.ref}` — dotted-path walk over `event.payload`,
  one key per `.` segment.
- Values stringify like JS: `null` → `"null"`, numbers/booleans as
  `String(v)`, arrays join with `,`, objects → `"[object Object]"`.
- A miss — absent key, non-object step, empty or unclosed
  placeholder, an interior containing `{` — stays **verbatim**
  (`{missing}` in the key is a literal, still deterministic).
- `{{` emits a literal `{` and rescans: `{{repo}}` → `{a/b}`.

## Queue semantics

`Queue::submit(event)` resolves the key and returns a `Decision`
(`key`, `disposition`, `cancel_active`):

- **Dedupe** applies to *pending* jobs only — under `latest-wins` the
  new event replaces the same-key pending job in place and the
  displaced `Job` comes back in
  `Disposition::ReplacedPending(job)` for observability. The *active*
  run is never deduped: same-key events while it runs queue another
  run (or cancel it, with `cancel_running`).
- **Cancel** is a one-shot signal: the first `cancel_running` submit
  while a run is active reports `cancel_active` and stashes the
  superseded `Job`, handed out once by `take_cancel()`. `finish()`
  clears an unread signal — a run that ended is moot to cancel.
- **Drain** is sequential (the gate case): `drain()` returns `None`
  while a job is active and otherwise pops the front job and marks it
  active; `finish()` releases the slot. Pending jobs iterate in
  drain order via `pending()`.

## Ingress injection channel

External events can't call `LocalBus::publish` directly — it's
`&mut` and the runner is single-threaded, so a blocking producer (a
webhook listener, a WAL poller) would stall the loop (audit I-014).
The channel splits that (`peregrin-engine`, `std::sync::mpsc` — the
engine is synchronous):

- `peregrin_engine::ingress()` returns `(Injector, Ingress)`.
- **`Injector`** is the `Send + Sync`, cloneable producer half — any
  thread or spawned ingress driver pushes `Event`s without `&mut`
  access to anything. `inject` fails with `InjectError` (carrying
  the event back) once the run's receiver is dropped.
- **`Ingress`** is the runner's receiver — `Runner::with_ingress`
  attaches it and the run loop drains it into `pending` once per
  iteration.

Ordering guarantee: injected events serialize **after** every event
already pending at the drain — including the triggers of the step
currently executing — and **before** anything nodes emit afterward.
A `workflow.cancel` pushed through the injector is honored at the
next iteration boundary, exactly like a node-emitted one (it is
consumed by the loop, not published — the log shows
`workflow.cancelled`).

Two edge rulings:

- Events pushed **before** `run()` starts are delivered — the runner
  drains on the first iteration too (`RunSupervisor` uses this to
  deliver the trigger event itself).
- Events arriving while the *last* step executes still land: the
  loop drains **before** its exit check, so only an event arriving
  after the final iteration can be dropped — a quiescent run exits;
  the channel does not keep it alive.

## `RunSupervisor` composition

`peregrin_node::supervisor::RunSupervisor` is the composable wrapper
the daemon drives: `submit(event)` → `SubmitOutcome { decision,
start, cancelled_active }`; `finish()` → `Option<PendingRun>`.

- `start` is `Some(PendingRun)` only when no run is active — it
  carries a fresh `(injector, ingress)` pair for its run with the
  trigger event already queued on it; hand `ingress` to
  `Runner::with_ingress`, clone `injector` for external producers.
- Under `cancel_running`, a superseding submit pushes
  `Event::cancel("superseded by a newer trigger for key '<key>'")`
  into the active run's own ingress channel — the cancellation
  surface the single-threaded runner actually honors.
- The runner itself stays `!Send` (Rc internals) — runs execute on
  the loop thread; the channel is the entire cross-thread input
  surface.
