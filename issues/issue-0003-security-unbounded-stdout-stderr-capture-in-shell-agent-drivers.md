+++
id = "I-003"
title = "security: unbounded stdout/stderr capture in shell/agent drivers"
status = "open"
created = "2026-11-03"
priority = "medium"
+++

read_pipe read_to_string() has no cap in peregrin-driver-shell lib.rs:142 and peregrin-driver-agent lib.rs:190 — a child writing unbounded output (e.g. 'yes') inflates memory until the timeout kill (~30s at MB/s+ = GBs). Fix: bounded capture that drains-and-discards past a cap.
