# Abstraction via event types

## The insight

In OOP, an interface separates _what_ (method signatures) from _how_
(implementation). The interface is the contract; the impl is swappable.

In an event system, **the events ARE the interface.** The event types
flowing between nodes are the contract. `on`/`emits` is the signature.
There is no need for a separate "interface" concept — the event types
themselves are the abstraction boundary.

## What this means

A "component" is not a separate kind of thing. It is just a named node with
a default implementation that you can override. The "interface" part is
the event contract (`emits: [notify.requested]`), the "impl" part is the
behaviour (`driver: shell, with: {cmd: ...}`). Both live on the same object.

But you do not even need that. **Pub/sub is the abstraction.** The emitter
does not know any subscriber exists. The event type is the only shared
contract. There is nothing to soften — decoupling is a property of the medium.

## In practice

```yaml
# The abstract intent — just emits an event
- id: request-notify
  on: [build.done]
  emits: [notify.requested]

# Implementation 1: desktop notification
- id: notify-desktop
  on: [notify.requested]
  driver: shell
  params:
    cmd: "notify-send '${event.payload.message}'"

# Implementation 2: also log to file (just add another subscriber)
- id: notify-log
  on: [notify.requested]
  driver: shell
  params:
    cmd: "echo '${event.payload.message}' >> /var/log/builds.log"
```

- **Swap implementations** by changing one subscriber node. Zero changes
  to emitters.
- **Multiple implementations** by adding subscribers. Zero coordination.
- **No implementation at all** — the event fires into the void. That is
  fine, not an error.
- **Test in isolation** by emitting an event and asserting what subscribers
  do. The event _is_ the test fixture.

## Why not OOP inheritance?

We tried a `Component` class with `uses` references. It created a parallel
type hierarchy that drifted — every new Node field had to be mirrored in
Component. The merge logic duplicated what pub/sub already does.

The event type is a better abstraction boundary than a class hierarchy
because it is **already decoupled.** The emitter does not import or reference
the subscriber. They share only a string (the event type). That is the
loosest possible coupling, and it is free.
