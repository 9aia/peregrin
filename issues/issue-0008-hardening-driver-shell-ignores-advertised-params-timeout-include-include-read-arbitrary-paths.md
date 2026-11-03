+++
id = "I-008"
title = "hardening: driver-shell ignores advertised params.timeout; include:/${include()} read arbitrary paths"
status = "done"
created = "2026-11-03"
priority = "low"
+++

input_schema advertises timeout but invoke uses hardcoded 30s — honor it (clamped). Advisory: yaml include: + ${include()} resolve absolute/../ paths into file reads — equal-or-weaker than driver: shell powers; document campaigns-as-code trust boundary. Also http driver params.url is unconstrained SSRF-by-design — strict capabilities (http.allow/deny globs) are the mitigation.

## Resolution

params.timeout honored (≤600s cap) in 841c9ac8. include:/${include()}/${file()} path traversal + driver-http SSRF documented as by-design — campaigns are code; strict capabilities (shell/file/http allow-deny globs) are the enforcement mechanism.
