---
layout: home

hero:
  name: peregrin
  text: Orchestrate your agents
  tagline: Event-driven orchestration for agent workflows — nodes, campaigns, and remote transport across your machines.
  actions:
    - theme: brand
      text: Get started
      link: /tutorials/getting-started
    - theme: alt
      text: Architecture
      link: /explanation/architecture

features:
  - title: Nodes
    details: A daemon per machine hosts the session plane (via sepia serve) and campaign drivers — one install, two capability planes.
  - title: Campaigns
    details: Declarative workflows over agents — graph-walking engine, YAML loader, templating, durable runs.
  - title: Hub
    details: Aggregation across nodes — merged entity/session views, sync, lock arbitration, and fleet dispatch.
---

## What it is

Peregrin is event-driven orchestration for agent workflows: a daemon
per machine (`peregrin-node`) hosts the session plane and campaign
engines, a headless `peregrin-hub` aggregates nodes into one fleet
view, and declarative YAML campaigns turn events (webhook pushes,
schedules, CI verdicts) into bounded agent work.

## Why

Coding agents multiply fast; the hard part is deciding *who may act
on a session, when, and on which machine*. Peregrin is the policy
layer — sepia does things *to* sessions, peregrin decides
*who/when/where* — with durable, queue-superseded runs and an
explicit capabilities/secret boundary so AI-governed workflows stay
safe.

## Boundary

- **Owns:** the session/entity planes' integration side, campaigns,
  drivers, events, remote transport, the hub gateway.
- **Doesn't own:** session mechanism ([sepia](https://9aia.com/sepia/)),
  the generic entity substrate ([hypha](https://github.com/9aia/hypha)),
  bundle distribution and service lifecycle
  ([tamias](https://9aia.com/tamias/)).

## Where to go next

| You want to… | Go to |
| --- | --- |
| Learn by doing — first node, first campaign | [Tutorials](/tutorials/getting-started) |
| Do a specific task (run a node, serve campaigns, deploy a bundle) | [How-to](/how-to/) |
| Understand the design (planes, drivers, safety, threat model) | [Explanation](/explanation/) |
| Look up a verb, env var, or schema field | [Reference](/reference/) |
