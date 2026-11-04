# Threat model: who trusts whom, and what happens when trust is wrong

This document complements [Safety](safety.md), which covers _how_
capability enforcement works. This document covers _who_ is trusted,
_what_ each role must guarantee, _what happens when a role is
wrong_, and _what is confined vs. not confined_. If you are building
a production deployment on peregrin, read this before trusting the
safety claims.

## The roles

| Role | Who/what | Trust level | What it controls |
| --- | --- | --- | --- |
| **Campaign author** | Human who writes the campaign YAML and its `capabilities:` | Trusted | The capability boundary (the "constitution") |
| **Operator** | Human who runs `install`/`create`/`deploy` and feeds the secrets record | Trusted | Which bundles exist, which repos are watched, the secret *values* |
| **Generator** | AI that creates new node specs | Untrusted | Node specs (data) |
| **Reviewer** | AI that reduces the generator's capabilities before injection | Semi-trusted | Which self-attested caps survive |
| **Engine** | `Runner::execute_node` — the dispatch-time check | Trusted (mechanical) | Whether a node's resolved `params:` may run |
| **Driver** | shell, http, agent, git, … — executes the node | Untrusted (runs after check) | Side effects within the granted capability surface |
| **Ingress** | `github-webhook`, `sturn-poll` — pushes events into a live run | Untrusted input, trusted plumbing | Which external events enter the graph |
| **Agent runtime** | The agent inside a `driver: agent` node | Untrusted | Behaviour within its own sandbox/permission mode |

## Trust boundary: the campaign caps are the line

The **campaign-level `capabilities:` block** is the trust boundary.
It is declared by the (trusted) author and cannot be widened by any
AI role. Here is why that matters:

### The self-attestation problem

Node-level capabilities with `merge: false` (the default)
**replace** the campaign's caps. So a generator can self-attest
`shell.allow: ["*"]` and, if left unchecked, that node would bypass
the campaign's narrower `shell.allow: ["echo *"]`. This is not a bug
— it is the mechanism that lets a node declare "I only need eslint,
not the full git/npm baseline." But it means **self-attested caps
are not confinement; they are a request.**

### What the reviewer must guarantee

The reviewer's job is **capability reduction**: strip the
generator's self-attested `capabilities` key so the node falls back
to the campaign's boundary, or replace it with a _narrower_ grant.
The reviewer must:

1. **Remove or narrow** every self-attested capability that exceeds
   the campaign boundary.
2. **Not introduce** capabilities the campaign does not grant.

The reviewer does **not** need to be perfect. It needs to be
conservative: when in doubt, strip. Stripping too much makes a node
fail safely.

### What happens when the reviewer is wrong

**Reviewer is too strict (strips too much):** the node is blocked by
the caps, emits `capability.violation`, and does not run. **Blast
radius: the node does not execute. No side effects. This is the
safe failure mode.**

**Reviewer is too permissive (leaves a dangerous cap in place):** if
the reviewer leaves `shell.allow: ["*"]` on a node with
`params.cmd: "rm -rf /"`, and `merge: false` replaces the campaign
caps, the node's caps now allow `rm`. **Blast radius: the node
executes with the self-attested caps. The campaign boundary is
bypassed for that node.**

## Open gap: inject narrowing

The single most important unfinished safety item, inherited from v1:
**`ctx.inject` does not refuse to inject a node whose capabilities
exceed the injecting node's.** `LocalBus::inject` registers specs
as-is. The dispatch-time capability check still runs on the injected
node — the *campaign* boundary holds — but if the reviewer left
broad self-attested caps in place, the injected node can carry
wider caps than the injector's own. Until narrowing ships, the safe
pattern is to **strip caps, not narrow**.

## What is confined vs. not confined

### Confined (the engine guarantees this)

- **Which shell commands run** — `params.cmd` is checked before the
  shell starts (deny-then-allow globs).
- **Which URLs are fetched** — `params.url` is checked before the
  HTTP request.
- **Which agent runners are invoked** — `params.runner` against
  `agents` (exact match).
- **Which template effects fire** — `${env()}`/`${file()}`/
  `${shell()}` calls need a `capabilities.<name>` grant; strict mode
  glob-matches the argument.
- **Which secrets are visible** — `capabilities.secrets` gates
  `${secret.*}`, `ctx.get_secret`, and every `*_env` driver param;
  the name itself is glob-matched under strict.
- **Which events enter a run** — ingress drivers verify before they
  inject: `github-webhook` demands a per-repo HMAC signature and a
  watched-repo allowlist; unknown repos and bad signatures are
  rejected upstream of the graph.
- **Where webhooks bind** — under `default_policy: deny`, the
  `webhook` capability policy checks `ports`/`hosts`/`paths` and can
  require auth config.

### Not confined (the engine does NOT guarantee this)

- **What a shell command does once it starts.** The capability
  confines the _command surface_, not the command's effects —
  `allow: ["make *"]` still runs whatever the Makefile does.
- **What an agent does within its own sandbox.** The engine confines
  *that* the agent runs and its permission mode, not its runtime
  behaviour.
- **OS-level escape.** No seccomp, firejail, or container
  confinement — run untrusted campaigns in a container or VM.
- **The secrets record source.** `RunOptions.secrets` is fed by the
  caller; the engine gates *reads*, but who populates the record is
  an operator decision. The ops verbs resolve names through env →
  keyring → `~/.config/peregrin/env` — the run-side feed of that
  chain is pending wiring.

**Summary:** the engine confines the **declared interface**. It does
not confine the **runtime behaviour** within that interface.

## Supply chain: the bundle is code

`peregrin install <ref>` fetches a bundle and `create` materializes
a config that *executes* it — a bundle is remote code running on
your node, with your secrets record. The mitigations are pinning,
not sandboxing:

- `#<ref>` pins to a branch/tag/sha; the resolved commit sha is
  always recorded, and `#sha256:` tarball refs are content-pinned.
- `peregrin verify <name>` digest-checks installed contents against
  the recorded sha.
- A bundle can declare `capabilities:` — treat a bundle's requested
  caps the way you treat a mobile app's permission list. Audit
  `peregrin info <name>` before `create`.

## Practical guidance

If you are deploying peregrin campaigns today:

1. **Set campaign-level `default_policy: deny` + `strict: true`**
   and declare only the caps the campaign needs — strict is what
   turns `capability.violation` into `node.failed` and glob-matches
   secret names.
2. **Strip, don't narrow.** Remove the `capabilities` key from
   generated/injected nodes.
3. **Assume the reviewer is fallible.** Design campaigns so a
   too-permissive review fails _safe_ (harmless or reversible), not
   _unsafe_.
4. **Don't rely on the engine for OS-level confinement.** Run
   untrusted campaigns in a container or VM.
5. **Subscribe to `capability.violation`.** Log it, alert on it, and
   treat it as a signal that the reviewer or generator is
   misbehaving.
6. **Keep secrets out of the graph.** Params and configs carry
   secret *names* (`token_env`, `secret_env`, `ssh_env`); the event
   log is deliberately shareable — a secret that reaches a payload
   is a secret in your history file.

## See also

- [Safety](safety.md) — the enforcement mechanics
- [Scope resolution](../reference/scope-resolution.md) — the
  `${secret.*}` gate contract
- [Distribution](../reference/distribution.md) — bundle refs, pins,
  `verify`
