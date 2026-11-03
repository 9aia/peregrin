+++
id = "I-005"
title = "security: peregrin-keyring passes unvalidated secret names to secret-tool argv"
status = "open"
created = "2026-11-03"
priority = "low"
+++

name values starting with '-' parse as flags; empty names hit undefined behavior. Fix: reject empty/'-'-leading names.
