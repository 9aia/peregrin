//! peregrin-bus — the campaign event system: glob-pattern matching
//! (`matches`/`capture_wildcards`), the [`EventBus`] port the runner
//! publishes through, the in-process [`LocalBus`] impl (port of the TS
//! `EventBusLive`), and the [`Observer`] hook port. Remote fan-out
//! lives on the `peregrin-remote` transport.

mod bus;
mod observer;

pub use bus::{EventBus, LocalBus, PublishResult};
// Pattern matching lives in `peregrin-core` (pure domain — the
// template scope pass needs it too); re-exported for compatibility.
pub use observer::{NoopObserver, Observer, Span};
pub use peregrin_core::{capture_wildcards, matches};
