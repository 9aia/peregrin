+++
id = "I-017"
title = "bupha supersede teardown races the dying run"
status = "open"
created = "2026-11-05"
owner = "devin"
priority = "med"
+++

Observed 2026-11-05: superseding a v2 run reclaims the previous run worktree while its child is still executing, so the dying run logs spurious I/O errors ("No such file or directory" writing target/deps, "Directory not empty" on delete) and a leaked worktree (bupha-9aia-peregrin-v2-b045b89b2729) was left behind. Superseded runs also post state `error` (GitHub has no `cancelled` state) — that part is by design. Fix: on supersede, kill the child and wait for exit before reclaiming the worktree. Lives in the bupha repo; peregrin ci-gate should adopt the same ordering when it replaces bupha (P8).
