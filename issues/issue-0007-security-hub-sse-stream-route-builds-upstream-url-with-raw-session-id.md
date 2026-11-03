+++
id = "I-007"
title = "security: hub SSE stream route builds upstream URL with raw session id"
status = "done"
created = "2026-11-03"
priority = "low"
+++

peregrin-hub proxy.rs sse_session_stream concat {id} into the node URL unencoded — ids containing ?/#/@ smuggle query/authority into the upstream request. Fix: percent-encode the segment.

## Resolution

Fixed on security/audit-fixes (ee40b83a): session id percent-encoded via peregrin_sync::client::encode_segment.
