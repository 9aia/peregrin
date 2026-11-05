+++
id = "I-018"
title = "campaign secret resolution has no ambient-env or keyring leg"
status = "open"
created = "2026-11-05"
owner = "devin"
priority = "medium"
+++

peregrin-driver ExecutionContext::get_secret_or reads only the run secrets record; peregrin campaign run seeds it from ~/.config/peregrin/env (the unit loads it via EnvironmentFile). So `export FOO=... && peregrin campaign run` does NOT resolve FOO, though the docs promised env -> OS keyring -> file. Verified by the first-gate e2e (webhook node reported the secret unresolved until the name was added to the env file). Docs now describe the real behaviour; the fix is to add the ambient-env fallback (and/or the keyring leg) with tests.
