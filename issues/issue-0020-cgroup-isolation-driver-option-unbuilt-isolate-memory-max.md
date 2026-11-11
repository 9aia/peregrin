+++
id = "I-020"
title = "cgroup isolation driver option unbuilt (isolate/memory_max)"
status = "open"
created = "2026-11-07"
priority = "med"
+++

# cgroup isolation driver option unbuilt (isolate/memory_max)

Status: **open** · Created: 2026-11-07

## Description

P2 leftover. `peregrin-cli/src/import.rs:459-460` warns "cgroup isolation is a driver option not yet wired into driver-git"; zero cgroup/memory_max impl in workspace. Needs a driver-port option + impl on the spawn path (driver-git/shell). The OOM-containment story for agent runs.

## Checklist

- [ ] See Description

## Resolution
