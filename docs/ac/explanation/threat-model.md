# Threat model: who trusts whom, and what happens when trust is wrong

This document complements [safety.md](safety.md), which covers _how_
capability enforcement works. This document covers _who_ is trusted, _what_
each role must guarantee, _what happens when a role is wrong_, and _what is
confined vs. not confined_. If you are building a production deployment on
agent-campaign, read this before trusting the safety claims.

## The roles

| Role                | Who/what                                                      | Trust level                  | What it controls                                   |
| ------------------- | ------------------------------------------------------------- | ---------------------------- | -------------------------------------------------- |
| **Workflow author** | Human who writes the workflow YAML and its `capabilities:`    | Trusted                      | The capability boundary (the "constitution")       |
| **Generator**       | AI that creates new nodes                                     | Untrusted                    | Node specs (data)                                  |
| **Reviewer**        | AI that reduces the generator's capabilities before injection | Semi-trusted                 | Which self-attested caps survive                   |
| **Engine**          | `NodeExecutor.run_node` — the dispatch-time check             | Trusted (mechanical)         | Whether a node's `with:` is allowed to run         |
| **Driver**          | shell, python, http, agent, webhook — executes the node       | Untrusted (runs after check) | Side effects within the granted capability surface |
| **Agent runtime**   | The LLM inside a `driver: agent` node                         | Untrusted                    | Behaviour within its sandbox/permission_mode       |

## Trust boundary: the workflow caps are the line

The **workflow-level `capabilities:` block** is the trust boundary. It is
declared by the (trusted) workflow author and cannot be widened by any AI
role. Here is why that matters:

### The self-attestation problem

Node-level capabilities with `merge: false` (the default) **replace** the
workflow's caps. So a generator can self-attest `shell.allow: ["*"]` and,
if left unchecked, that node would bypass the workflow's narrower
`shell.allow: ["echo *"]`. This is not a bug — it is the mechanism that lets
a node declare "I only need eslint, not the full git/npm baseline." But it
means **self-attested caps are not confinement; they are a request.**

### What the reviewer must guarantee

The reviewer's job is **capability reduction**: strip the generator's
self-attested `capabilities` key so the node falls back to the workflow's
boundary, or replace it with a _narrower_ grant. The reviewer must:

1. **Remove or narrow** every self-attested capability that exceeds the
   workflow boundary.
2. **Not introduce** capabilities the workflow does not grant.

The reviewer does **not** need to be perfect. It needs to be conservative:
when in doubt, strip. Stripping too much makes a node fail safely.

### What happens when the reviewer is wrong

**Reviewer is too strict (strips too much):** The node is blocked by the
caps, emits `capability.violation`, and does not run. **Blast radius: the
node does not execute. No side effects. This is the safe failure mode.**

**Reviewer is too permissive (leaves a dangerous cap in place):** If the
reviewer leaves `shell.allow: ["*"]` on a node with `with.cmd: "rm -rf /"`,
and `merge: false` replaces the workflow caps, the node's caps now allow
`rm`. **Blast radius: the node executes with the self-attested caps. The
workflow boundary is bypassed for that node.**

Two mitigations exist today, one is planned:

1. **Today — the reviewer strips caps by default.** The
   capability-confinement example demonstrates this. As long as the reviewer strips, the workflow boundary
   holds.

2. **Planned — inject-time narrowing.** `Runner.inject` will refuse to
   inject a node whose effective capabilities exceed the injecting node's.
   This is the object-capability rule enforced mechanically: **delegation
   only narrows.** See [safety.md § Delegation narrows](safety.md#delegation-narrows-never-widens).

Until inject narrowing ships, **the reviewer's correctness is a
load-bearing trust assumption.**

## What is confined vs. not confined

### Confined (the engine guarantees this)

- **Which shell commands run** — `with.cmd` is checked before the shell starts.
- **Which URLs are fetched** — `with.url` is checked before the HTTP request.
- **Which agent runners are invoked** — `with.runner` is checked.
- **Which Python handlers run** — `node.handler` is checked before the handler is called.
- **Where webhooks bind** — `with.port`/`host`/`path` are checked.
- **Which secrets are visible** — `Context.secrets` is filtered.

### Not confined (the engine does NOT guarantee this)

- **What a shell command does once it starts.** The capability confines the
  _command surface_, not the command's effects.
- **What a Python handler does internally.** The capability confines _which
  handler runs_, not what it does.
- **What an agent does within its sandbox.** The engine confines that the
  agent is allowed to run and its permission mode, but not its runtime
  behaviour.
- **OS-level escape.** No seccomp, firejail, or container confinement today.
  `--sandbox subprocess` is planned (see [safety.md § Build order](safety.md#build-order)).

**Summary:** the engine confines the **declared interface**. It does not
confine the **runtime behaviour** within that interface.

## Open gap: inject narrowing

The single most important unfinished safety item: **`Runner.inject` does
not yet refuse to inject a node whose capabilities exceed the injecting
node's.** Today, a reviewer node can inject a node with broader caps than
its own. The capability check still runs on the injected node at dispatch
time, so the _workflow_ boundary holds — but if the reviewer left broad
self-attested caps in place, the injected node can bypass the workflow
boundary for its own execution.

Inject narrowing closes this: the engine refuses the injection. Until it
ships, the safe pattern is to strip caps, not narrow.

## Practical guidance

If you are deploying agent-campaign today:

1. **Set workflow-level `default_policy: deny`** and declare only the caps
   the workflow needs.
2. **Strip, don't narrow.** Remove the `capabilities` key from generated nodes.
3. **Assume the reviewer is fallible.** Design workflows so that a too-
   permissive review fails _safe_ (harmless or reversible), not _unsafe_.
4. **Don't rely on the engine for OS-level confinement.** Run untrusted
   workflows in a container or VM.
5. **Subscribe to `capability.violation`.** Log it, alert on it, and treat
   it as a signal that the reviewer or generator is misbehaving.
