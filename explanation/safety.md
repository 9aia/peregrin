# Safety: how AI-governing-AI is kept safe

This project's thesis is that AI can generate, review, evaluate, and
improve its own workflows — safely. "Safely" has a concrete meaning
here, not a vibe. This document defines it and explains how the
engine enforces it.

## What "safe" means

Two independent properties, both required:

1. **Bounded authority** — a node can only do what it is explicitly
   granted.
2. **Bounded execution** — a workflow cannot run forever or
   accumulate without limit.

If either fails, "AI-governing-AI" degrades to "AI does whatever it
wants."

## Bounded authority (what a node can do)

Every node carries a `capabilities:` grant — a data field, not a
code contract. Capabilities are declared at two levels:

### Campaign-level defaults

```yaml
name: my-campaign
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

### Node-level (overrides or merges)

```yaml
- id: lint
  driver: shell
  capabilities:
    merge: false        # default — node lists replace campaign's
    shell:
      allow: ["eslint *"]
    secrets: [API_KEY]
```

```yaml
- id: deploy
  driver: shell
  capabilities:
    merge: true         # node lists union with campaign's
    shell:
      allow: ["docker *"]
```

### Merge semantics

Implemented by `CapabilityChecker::merge` (`peregrin-engine`):

| Field type | `merge: false` (default) | `merge: true` |
| --- | --- | --- |
| Top-level lists (`secrets`, `agents`, `urls`) | Node's list **replaces** campaign's | **Unions** |
| Dict-list keys (`shell`, `http`, `python`, `webhook`) — their `allow`/`deny`/`ports`/`hosts`/`paths` | Node's list **replaces** | **Unions** |
| Scalars (`default_policy`), other keys | Node's value wins | Node's value wins |
| Undeclared fields | **Inherit** campaign default | **Inherit** campaign default |

Replace-by-default is the safe choice for deny-all: a node only gets
what it explicitly declares. Merge is opt-in for nodes that need to
extend the campaign's baseline. See
[Campaign schema § Capability merge](../reference/campaign-schema.md).

### Enforcement

Enforcement lives in `Runner::execute_node` — the single dispatch
point every fire crosses, *before* any driver is invoked and *after*
`${...}` template resolution (so the final, resolved command or URL
is what gets checked):

- **Shell** — `params.cmd` matched against `shell.allow`/`shell.deny`
  (anchored globs: `*` any run, `?` one char — neither crosses line
  terminators). Deny wins; no allow list under deny-all blocks
  everything.
- **HTTP** — `params.url` against `http.allow`/`http.deny`.
- **Agent** — `params.runner` must be in `agents` (exact match).
- **Python** — `params.handler` against `python.allow`/`python.deny`
  (`module:func` — `allow: ["myapp:*"]` grants any function from a
  module). The check is ported; the `python` driver itself is not
  yet.
- **Webhook** — `params.port`/`host`/`path` checked against
  `webhook.ports`/`hosts`/`paths` (`require_auth: true` additionally
  demands `auth.type` + `auth.secret` in params).

A violation emits `capability.violation` (payload `{node,
violations}`), the node is **not** executed, and under `strict:
true` a `node.failed` follows — see
[Event envelope](../reference/event-envelope.md).

### Default policy

- `default_policy: deny` — nothing runs unless explicitly granted.
- `default_policy: allow` — enforcement off entirely.
- Unset — the fallback is `deny` under `strict: true`, `allow`
  otherwise. A campaign with no `capabilities:` at all enforces
  nothing.

## Secret gating — names, never values

Secrets reach a run through `RunOptions.secrets` — a record the
operator feeds, never the campaign file. Three surfaces read it, all
through the same gate:

- **`${secret.NAME}`** in templates — resolves only when the merged
  `capabilities.secrets` grant is present and not `false`; under
  `strict` the *name* must also match the grant (`true` = all, a
  list = name globs, `{allow, deny}` = deny-then-allow). A lookup
  miss substitutes `""` and lands in the `missing_secrets` audit
  ledger — which fails the strict audit with `missing secrets: …`.
- **`ctx.get_secret(NAME)`** in drivers — identical contract; the
  ledger is shared.
- **`*_env` params** (`token_env`, `secret_env`, `ssh_env`) — drivers
  take the *name* of a secret and resolve it through the same gate.
  Config carries names; values live only in the run's record. That
  is why a shipped bundle can reference `GITHUB_TOKEN` without ever
  containing a token.

`CapabilityChecker::filter_secrets` — the TS half that filtered the
record itself down to the allow-listed keys — is ported but not yet
called: today the gate is point-of-use, so the full record reaches
`ctx.secrets` and every read is gated instead.

> **Secret resolution chain.** `ctx.resolve_secret` (the lookup behind
> `${secret.NAME}` and `*_env` params) consults, in order: the run's
> secrets record → ambient process env → fallback stores
> (`RunOptions.secret_stores`; the stock node wires OS keyring via
> `hypha-keyring`, then `~/.config/peregrin/env` via
> `hypha-dotenv`). The keyring leg is *declared-only* — it is
> consulted solely for names matching the merged
> `capabilities.secrets` grant, so undeclared names never spawn
> `secret-tool` or reach OS credentials. A hit on any leg suppresses
> the `missing_secrets` ledger entry; only a full-chain miss records.

## Bounded execution (how long / how much)

- **Per-node:** `max_fires` caps firings (default 1), `until` stops
  on a condition event, `retire_on` removes the node.
- **Campaign:** `stop_on` ends the run; `supersede.cancel_running`
  cancels a run a newer trigger replaces.
- **Failures** are contained: retry with backoff, dead-letter queue,
  circuit breaker — [Resilience](resilience.md).
- **Not ported yet:** the automatic cycle detector and global
  `max_events`/run `timeout` — see
  [Loop protection](loop-protection.md).

## How enforcement is guaranteed

The guarantee comes from *where* the check lives, not from driver
cooperation:

- The capability check runs inside `Runner::execute_node`, the only
  path from "a node is ready" to "a driver executes."
- A driver cannot bypass it because the driver is invoked *by* the
  runner, after the check passes.
- Template-time effects (`${env()}`, `${file()}`, `${shell()}` calls)
  go through the same grant mechanism — `capabilities.<name>`
  presence whitelists the call, strict mode matches the argument.

This is "hard sandbox," not "conventions handlers respect
voluntarily." A shell command can escape any YAML convention; it
cannot escape a check that refuses to start the shell.

## What this does NOT ensure

Being honest about the limits:

- **The capability check confines the declared interface, not the
  runtime behavior inside it.** A granted `shell.allow: ["make *"]`
  can still `rm -rf` inside the Makefile.
- **`ctx.inject` does not narrow.** An injected node's capabilities
  are *not* checked against the injecting node's — the dispatch-time
  check still runs (the campaign boundary holds), but self-attested
  node caps can exceed the injector's own. The safe pattern: strip
  `capabilities` off generated nodes, don't narrow. See
  [Threat model](threat-model.md#open-gap-inject-narrowing).
- **No OS-level confinement.** No seccomp, container, or subprocess
  sandboxing — run untrusted campaigns in a container or VM.

## Ingress trust surface

Ingress drivers are where untrusted input enters the graph; the
shipped ones verify before they inject:

- `github-webhook` — `X-Hub-Signature-256` HMAC-SHA256 per repo
  (secret *names* in `params.repos[].secret_env`, resolved through
  the secrets gate), watched-repo allowlist, delivery-id dedup,
  2 MiB body cap; unwatched repos, unverified signatures, and
  non-`push`/`pull_request` event types are rejected before an event
  is ever injected. `pull_request` deliveries are additionally
  action-filtered per route (`params.pull_request_actions`, default
  `opened,synchronize,reopened`) — a verified delivery for an
  unwanted action is a `200` no-op, never an injected event.
- `sturn-poll` — unauthenticated by design (it diffs git refs
  locally); `actor` is unattested `""`.

## See also

- [Threat model](threat-model.md) — who trusts whom, and blast
  radii when trust is wrong
- [Scope resolution](../reference/scope-resolution.md) — the
  `${secret.*}` contract
- [Campaign schema](../reference/campaign-schema.md) — the
  `capabilities` field and merge rules
