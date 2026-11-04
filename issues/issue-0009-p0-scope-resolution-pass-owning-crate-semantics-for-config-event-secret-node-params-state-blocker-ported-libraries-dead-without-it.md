+++
id = "I-009"
title = "P0: scope-resolution pass — owning crate + semantics for ${config|event|secret|node|params|state} (blocker: ported libraries dead without it)"
status = "done"
created = "2026-11-04"
priority = "medium"
+++

# P0: scope-resolution pass — owning crate + semantics for ${config|event|secret|node|params|state} (blocker: ported libraries dead without it)

Status: **open** · Created: 2026-11-04

## Description

What is wrong and steps to reproduce.

## Checklist

- [ ] Action item

## Resolution

Scope resolver shipped in peregrin-template (secrets gate+ledger, params cycle guard, :- depth cap 8, strict audit); ported libraries live — verified e2e via ci-gate run.
