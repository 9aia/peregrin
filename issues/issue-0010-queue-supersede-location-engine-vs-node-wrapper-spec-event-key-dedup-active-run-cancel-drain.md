+++
id = "I-010"
title = "queue/supersede location — engine vs node wrapper; spec event→key dedup, active-run cancel, drain"
status = "done"
created = "2026-11-04"
priority = "med"
+++

# queue/supersede location — engine vs node wrapper; spec event→key dedup, active-run cancel, drain

Status: **open** · Created: 2026-11-04

## Description

What is wrong and steps to reproduce.

## Checklist

- [ ] Action item

## Resolution

Resolved: node-side policy, not engine internals. New peregrin-queue crate (Supersede{key template, mode latest-wins|fifo, cancel_running}, Queue::submit/drain/finish/take_cancel, Job{seq,key,event}); RunSupervisor in peregrin-node composes it with the ingress channel. Commit 7d55fb44 on substrate/pl-003.
