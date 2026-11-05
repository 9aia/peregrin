//! `peregrin-node-api` — the node daemon's `/api/*` surface as a Rust
//! port, shared by `peregrin-hub` (its `SyncNodeApi` impl) and
//! `peregrin-web` (the leptos server fns that call through it). Pure
//! Rust — no leptos, no DOM — so a hub-only build never pulls the UI
//! stack.

pub mod api;
pub mod dto;
pub mod rewind;
pub mod targets;
pub mod testing;
