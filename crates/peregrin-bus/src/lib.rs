//! peregrin-bus — the campaign event system: glob-pattern matching
//! (`matches`/`capture_wildcards`), the [`EventBus`] port the runner
//! publishes through, the in-process [`LocalBus`] impl (port of the TS
//! `EventBusLive`), and the [`Observer`] hook port. Remote fan-out
//! lives on the `peregrin-remote` transport.

mod bus;
mod matching;
mod observer;

pub use bus::{EventBus, LocalBus, PublishResult};
pub use matching::{capture_wildcards, matches};
pub use observer::{NoopObserver, Observer, Span};
