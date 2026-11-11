+++
id = "I-018"
title = "campaign secret resolution has no ambient-env or keyring leg"
status = "done"
created = "2026-11-05"
owner = "devin"
priority = "med"
+++

peregrin-driver ExecutionContext::get_secret_or reads only the run secrets record; peregrin campaign run seeds it from ~/.config/peregrin/env (the unit loads it via EnvironmentFile). So `export FOO=... && peregrin campaign run` does NOT resolve FOO, though the docs promised env -> OS keyring -> file. Verified by the first-gate e2e (webhook node reported the secret unresolved until the name was added to the env file). Docs now describe the real behaviour; the fix is to add the ambient-env fallback (and/or the keyring leg) with tests.

## Resolution

ctx.resolve_secret chain: record -> ambient env -> keyring (declared-only) -> env file; wired via RunOptions.secret_stores + peregrin-node::campaign::default_secret_stores. Main 6dedd06a.
