+++
id = "I-006"
title = "security: sqlite stores (outbox/projection) and secrets-bearing files written with default umask"
status = "open"
created = "2026-11-03"
priority = "medium"
+++

Outbox::open (peregrin-outbox:124) and ProjectionStore::open (peregrin-sync:266) create db files ~0644 holding session prompts/payloads. Fix: chmod 0600 after open. NOTE: sepia-meta meta.json (VAPID private key, push subs, session meta) has the same issue upstream in sepia — reported, fix belongs there (flush rewrites via rename so it can't be patched from here).
