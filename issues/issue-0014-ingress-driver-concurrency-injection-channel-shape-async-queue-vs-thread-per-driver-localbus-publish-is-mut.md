+++
id = "I-014"
title = "ingress-driver concurrency — injection channel shape (async queue vs thread-per-driver); LocalBus::publish is &mut"
status = "done"
created = "2026-11-04"
priority = "med"
+++

# ingress-driver concurrency — injection channel shape (async queue vs thread-per-driver); LocalBus::publish is &mut

Status: **open** · Created: 2026-11-04

## Description

What is wrong and steps to reproduce.

## Checklist

- [ ] Action item

## Resolution

Resolved: std::sync::mpsc channel. peregrin_engine::ingress() returns (Injector, Ingress); Injector is Send+Sync+Clone for external producers, runner drains Ingress into pending once per loop iteration before its exit check. Commit 7d55fb44 on substrate/pl-003.
