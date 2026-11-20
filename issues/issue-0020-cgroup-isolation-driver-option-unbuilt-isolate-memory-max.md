+++
id = "I-020"
title = "cgroup isolation driver option unbuilt (isolate/memory_max)"
status = "done"
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

**Done** (verified 2026-11-20). `isolate`/`memory_max` are driver-shell
params: `spawn_command` wraps the child in `systemd-run --user
--scope -p MemoryMax=` (`crates/peregrin-driver-shell/src/lib.rs`),
missing `systemd-run` fails loudly rather than silently degrading.
The bupha importer maps runner `isolate`/`memory_max` onto inner
shell params (`import.rs`), and driver-git delegates through
`inner.driver` — so confinement reaches every spawn path that goes
through shell. Param-level (not a port trait option) is the right
locus: cgroup wrapping is a spawn concern, and only shell spawns
arbitrary commands.
