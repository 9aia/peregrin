//! `peregrin-node-api` — the node daemon's entity-plane surface as a
//! Rust port, shared by `peregrin-hub` (its `SyncEntityApi` impl) and
//! integrations/clients. Pure Rust — no leptos, no DOM, and
//! deliberately **kind-agnostic**: entity kinds, op kinds, and
//! `index_doc`/`payload` shapes belong to integrations
//! (`sepia-peregrin`), never to this crate (ADR-0091).

pub mod api;
pub mod dto;
pub mod testing;
