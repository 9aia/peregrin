//! Route surface — the session-side routes (`/api/sessions*`,
//! agents, config/projects compat) live in `sepia-http` since PL-027;
//! this module re-exports them so `peregrin_http::routes::*` paths
//! keep resolving, plus peregrin's own `events` ingest route.

pub use sepia_http::routes::*;

pub mod events;
