+++
id = "I-008"
title = "hardening: driver-shell ignores advertised params.timeout; include:/${include()} read arbitrary paths"
status = "open"
created = "2026-11-03"
priority = "low"
+++

input_schema advertises timeout but invoke uses hardcoded 30s — honor it (clamped). Advisory: yaml include: + ${include()} resolve absolute/../ paths into file reads — equal-or-weaker than driver: shell powers; document campaigns-as-code trust boundary. Also http driver params.url is unconstrained SSRF-by-design — strict capabilities (http.allow/deny globs) are the mitigation.
