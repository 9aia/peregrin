+++
id = "I-007"
title = "security: hub SSE stream route builds upstream URL with raw session id"
status = "open"
created = "2026-11-03"
priority = "low"
+++

peregrin-hub proxy.rs sse_session_stream concat {id} into the node URL unencoded — ids containing ?/#/@ smuggle query/authority into the upstream request. Fix: percent-encode the segment.
