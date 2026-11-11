+++
id = "I-003"
title = "security: unbounded stdout/stderr capture in shell/agent drivers"
status = "done"
created = "2026-11-03"
priority = "med"
+++

read_pipe read_to_string() has no cap in peregrin-driver-shell lib.rs:142 and peregrin-driver-agent lib.rs:190 — a child writing unbounded output (e.g. 'yes') inflates memory until the timeout kill (~30s at MB/s+ = GBs). Fix: bounded capture that drains-and-discards past a cap.

## Resolution

Fixed on security/audit-fixes (841c9ac8): both drivers drain pipes to EOF keeping ≤10MiB — test with 12MiB output passes.
