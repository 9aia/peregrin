+++
id = "I-002"
title = "security: drivers + ${shell()} leak full daemon env to spawned children"
status = "done"
created = "2026-11-03"
priority = "high"
+++

peregrin-driver-shell lib.rs:158 envs(std::env::vars()), peregrin-driver-agent lib.rs:174 build_env, and peregrin-template SystemEffects::shell all inherit the whole process env — secrets materialized into the node env (PEREGRIN_TOKEN etc.) leak into any campaign cmd. Live-repro'd: cmd='env' dumps PEREGRIN_TOKEN. Same class as arca's PDP exec fix. Fix: env allowlist + params.env/params.secrets.

## Resolution

Fixed on security/audit-fixes (841c9ac8): children now start from peregrin_core::child_env() allowlist + declared params.env/params.secrets — live repro verified (child env dropped 59→12 vars, no PEREGRIN_TOKEN). Regression tests added to shell/agent/template.
