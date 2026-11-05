//! Wire DTOs — moved to `peregrin-node-api` so `peregrin-hub` can use
//! them without the leptos stack. Re-exported here wholesale so the
//! existing `crate::dto::*` paths keep resolving.

pub use peregrin_node_api::dto::*;
