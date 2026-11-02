+++
id = "I-001"
title = "ureq recv_body is a total deadline — never set it on open-ended streams"
status = "done"
created = "2026-11-02"
owner = "devin"
priority = "medium"
labels = ["gotcha"]
+++

Fixed in 9c8f30d9e (`fix(sync): SSE streams must outlive ureq's total body deadline`).

## The trap

`ureq::ConfigBuilder::timeout_recv_body` reads like a per-read idle timeout. It is a **total deadline from when response headers finish** (`timings.rs`: deadline = `RecvResponse` time + budget). Once spent, `Connection::maybe_await_input` (`pool.rs`) returns `Error::Timeout` **without polling the socket** — every subsequent `read()` fails instantly.

Consequences on an open-ended stream (SSE, long-poll):

- Bytes arriving after the deadline (e.g. server heartbeats) are never read — the stream is dead regardless of liveness.
- Any caller that catches the timeout and retries (`EventStream::next_event`'s stall path; `proxy.rs::pump_lines`' ping liveness probe) hot-loops at ~100% CPU until its own silence bound expires — then reconnects and repeats.

Observed signature: `io: timeout: receive body` every ~`recv_body + stall` interval plus a pegged reader thread. The hub was burning ~46% of a core on this.

## The rule

Never set `recv_body` on streams. Use `timeout_recv_body(None)` and bound silence at the app layer — a watchdog re-armed on every received frame (see `connect_once` in `peregrin-sync/src/engine.rs`). Applies anywhere SSE rides ureq — node client, hub proxy agents, future consumers.
