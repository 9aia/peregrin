# Safety: how AI-governing-AI is kept safe

This project's thesis is that AI can generate, review, evaluate, and improve its
own workflows — safely. "Safely" has a concrete meaning here, not a vibe. This
document defines it and explains how the engine enforces it.

## What "safe" means

Two independent properties, both required:

1. **Bounded authority** — a node can only do what it is explicitly granted.
2. **Bounded execution** — a workflow cannot run forever or accumulate without
   limit.

If either fails, "AI-governing-AI" degrades to "AI does whatever it wants."

## Bounded authority (what a node can do)

Every node carries a `capabilities:` grant — a data field, not a code contract.
Capabilities are declared at two levels:

### Workflow-level defaults

```yaml
name: my-workflow
capabilities:
  default_policy: deny
  shell:
    allow: ["git *", "npm *"]
    deny: ["rm -rf *"]
  http:
    allow: ["https://api.github.com/*"]
    deny: ["http://*"]
  secrets: [API_KEY, DB_PASSWORD]
  agents: [devin, cline]
```

### Node-level (takes precedence)

```yaml
- id: lint
  driver: shell
  capabilities:
    merge: false
    shell:
      allow: ["eslint *"]
    secrets: [API_KEY]
```

```yaml
- id: deploy
  driver: shell
  capabilities:
    merge: true
    shell:
      allow: ["docker *"]
```

### Merge semantics

| Field type                           | `merge: false` (default)            | `merge: true`                          |
| ------------------------------------ | ----------------------------------- | -------------------------------------- |
| Lists (allow, deny, secrets, agents) | Node's list **replaces** workflow's | Node's list **unions** with workflow's |
| Scalars (default_policy)             | Node's value wins                   | Node's value wins                      |
| Undeclared fields                    | **Inherit** workflow default        | **Inherit** workflow default           |

Replace-by-default is the safe choice for deny-all: a node only gets what it
explicitly declares. Merge is opt-in for nodes that need to extend the
workflow's baseline.

### Enforcement

Enforcement lives in `NodeExecutor.run_node` (`node_executor.py`), at the
single dispatch point, _before_ any driver is invoked:

- **Shell check** — `with.cmd` is matched against `shell.allow` (glob patterns
  via `fnmatch`) and `shell.deny`. Deny always wins. If no allow list is
  declared under deny-all, everything is blocked.
- **HTTP check** — `with.url` is matched against `http.allow` and `http.deny`
  (glob patterns).
- **Agent check** — `with.runner` must be in `agents` (exact match).
- **Python check** — `node.handler` (or `with.handler`) is matched against
  `python.allow` and `python.deny` (glob patterns). Supports `module:func`
  notation — `allow: ["myapp:*"]` grants any function from `myapp`,
  `deny: ["os:*"]` blocks the entire `os` module.
- **Webhook check** — `with.port`, `with.host`, `with.path` are checked
  against `webhook.ports` (exact), `webhook.hosts` (exact), and
  `webhook.paths` (glob). `webhook.require_auth: true` blocks webhooks
  without auth config.
- **Secret check** — `Context.secrets` is filtered to only the allowed keys.
  Both `ctx.get_secret("X")` and `${secret.X}` template references resolve to
  empty for disallowed keys.

The check happens _after_ `${...}` template resolution (so the final command
or URL is checked) but _before_ `driver.invoke`. The dangerous code never
executes.

### Default policy

- **`default_policy: deny`** (the default) — nothing is allowed unless
  explicitly declared.
- **`default_policy: allow`** — opts out of enforcement entirely.
- **No `capabilities:` at all** — same as `allow`: no enforcement.

### Violation handling

When a node's resolved `with:` block violates its capabilities:

1. A `capability.violation` event is emitted.
2. The node is **not** executed.
3. In `strict` mode (runner flag), `node.failed` is also emitted.

### Delegation narrows, never widens

`inject` refuses to inject a node whose capabilities exceed the injecting
node's. A reviewer node with `shell.allow: ["echo *"]` cannot inject a node
with `shell.allow: ["rm *"]`. **AIs cannot grant capabilities they do not
have.**

_Note: inject narrowing is a planned feature. The capability check itself is
implemented today; inject-time narrowing is the next step._

## Bounded execution (how long / how much)

- **Per-node:** `max_fires` caps firings. `until` stops once a condition event
  exists. `retire_on` removes a node from the graph.
- **Graph-level:** cycle detection watches for repeating event sequences.
- **Global:** `max_events` and `timeout` are hard stops.

Failures are contained, not cascading: retry with backoff, dead-letter queue,
and circuit breaker.

## How enforcement is guaranteed

The guarantee comes from _where_ the check lives, not from handler cooperation:

- The capability check runs in `NodeExecutor.run_node`, the only path from
  "a node is ready" to "a driver executes."
- A handler cannot bypass it because the handler is invoked _by_ the driver,
  which is invoked _after_ the check passes.
- `inject` is the only way to add nodes at runtime, and it will apply the same
  narrowing rule (planned).

This is "hard sandbox" (Path B), not "soft sandbox" (Path A — conventions
handlers respect voluntarily). A shell command can escape any YAML convention,
but it cannot escape a check that refuses to start the shell in the first place.

## What this does NOT ensure

Being honest about the limits:

- **The `python` driver handler allowlist controls _which handler runs_, not
  _what the handler does internally_.** A Python handler can still do anything
  Python can. True handler confinement requires OS-level sandboxing.
- **No OS-level confinement.** A malicious handler could escape the engine
  process. True hard confinement is a later feature (`--sandbox subprocess`).

So the current state is: **safe at the workflow layer (all drivers covered),
not yet safe at the OS layer.**

## Webhook security

Webhooks receive untrusted input from the network.

1. **Capability enforcement** — `webhook.ports`, `webhook.hosts`,
   `webhook.paths`, and `webhook.require_auth` confine where the server binds.
2. **Authentication** — two modes:
   - `auth.type: header` — shared secret in a custom header, verified with
     constant-time comparison.
   - `auth.type: hmac` — HMAC-SHA256 signature verification. Optional
     `timestamp_header` + `replay_window` prevents replay attacks.
3. **Transport security** — `with.tls: {cert, key}` enables HTTPS.

Additionally, `with.max_body_size` (default 1MB) prevents memory exhaustion
from oversized request bodies.

## Build order

1. `principal` + `capabilities` on `Node` ✅
2. `_check_capabilities` in `NodeExecutor.run_node` ✅
3. `inject` narrowing — _next_
4. `Context.secrets` filtering ✅
5. `python.allow` / `python.deny` ✅
6. `webhook.ports` / `hosts` / `paths` / `require_auth` ✅
7. HMAC signature verification + TLS + body size limit ✅
8. `--sandbox subprocess` — closes the OS escape hatch (separate effort)

See also the [threat model](threat-model.md) for the full trust analysis.
