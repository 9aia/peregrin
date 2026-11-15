# Concepts

The domain vocabulary, each term defined once. For the wire shapes
themselves see [Reference](../reference/index.md).

## Campaign

A declarative workflow: a YAML graph of nodes (drivers, expressions,
subworkflows) plus ingress routes that map events onto it. Campaigns
are entities — `kind="campaign"` — with `campaign.start|pause|
resume|cancel|signal` ops, and `campaign serve` can host many of them
behind one webhook listener.

## Node / hub

A **node** (`peregrin-node`, `peregrin serve`) is the per-machine
daemon: it owns the session plane (via `sepia serve`) and the campaign
engine, and serves the `/api/*` surface. The **hub**
(`peregrin-hub`) is a thin `hypha-hub` gateway that merges every
configured node's entity/session views into one API and arbitrates
locks and fleet dispatch.

## Entity

The substrate's unit of state: `{id, kind, indexDoc}` plus
namespaced ops and opaque snapshots. Peregrin registers its kinds —
`sepia-session`, `campaign` — on the generic entity plane; it never
names a sepia type in substrate code.

## Plane

A capability surface with its own routes and ownership: the session
plane (sepia mechanism), the entity plane (generic substrate), the
integration planes (peregrin's sessions-as-entities, campaigns), and
the app plane (`peregrin-http` composing one listener, the hub
gateway). See [Architecture](architecture.md#planes).

## Driver

A node type that does work: `agent`, `shell`, `git`, `schedule`,
`github-webhook`, `sturn`, `subworkflow`, and friends. Drivers are
registered in one unified registry and declared in campaign YAML —
the full list and contracts are in [Drivers](drivers.md) and the
[CLI reference](../reference/cli.md).

## Event

The unit of flow: named payloads matched by `on:` patterns that wake
campaign nodes. Events are also state — see
[Events](events.md) and the [event graph model](event-graph-model.md).

## Bundle

A tamias package of type `peregrin-bundle`: a `main.yaml` campaign
plus a `provider` manifest declaring services and ingress. Installed
and deployed through tamias, never by peregrin itself — see
[Run the docs-deploy bundle](../how-to/run-the-docs-deploy-bundle.md).

## Capability / secret grant

`capabilities:` on a campaign bounds what its drivers may do;
`capabilities.secrets` names exactly which secret names the
ingress/status drivers may resolve. A name that doesn't resolve
fails the node rather than silently degrading — see
[Safety](safety.md).

## Run / supersede

A **run** is one execution of a campaign. The queue policy is
latest-wins: a newer event on the same `{repo}:{ref}` key cancels the
in-flight run (`cancelled`/superseded) — see
[Queue & supersede](../reference/queue-supersede.md).
