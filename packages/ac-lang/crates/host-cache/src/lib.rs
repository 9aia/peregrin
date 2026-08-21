//! # ac-host-cache
//!
//! Caching host for cross-machine resumability.
//!
//! When a VM evaluation yields (via `Error::Yield`), the caller
//! captures a [`Continuation`][ac_vm::Continuation] and the yielded
//! value. To resume on another machine, the new host needs to
//! return results for calls that were already made — without
//! re-executing them.
//!
//! This crate provides:
//!
//! - [`CallCache`] — a serializable cache mapping `(function, args)`
//!   keys to result values. Can be serialized to JSON and sent
//!   across machines.
//! - [`CacheHost`] — a [`Host`][ac_vm::Host] implementation that
//!   checks the cache before dispatching to registered functions.
//!   Uncached calls fall through to the function; cached calls
//!   return instantly.
//!
//! ## Example: server → client migration
//!
//! ```ignore
//! use ac_host_cache::{CacheHost, CallCache};
//! use ac_vm::{Vm, Context, EvalResult, Continuation};
//! use ac_runtime::RuntimeValue;
//!
//! // Server: register functions, evaluate, hit a yield
//! let mut host = CacheHost::new();
//! host.register("add", Box::new(AddFunction));
//! host.register("fetch", Box::new(FetchFunction)); // yields
//! let mut vm = Vm::new(host);
//! let result = vm.eval_resumable(&expr, &mut ctx, None);
//! match result {
//!     EvalResult::Yielded { value, continuation } => {
//!         let cont_json = continuation.to_json();
//!         let cache_json = vm.host().cache().to_json();
//!         // Send cont_json + cache_json to client
//!     }
//!     _ => {}
//! }
//!
//! // Client: restore cache, resume
//! let cache = CallCache::from_json(&received_cache_json);
//! let mut host = CacheHost::with_cache(cache);
//! host.register("add", Box::new(AddFunction));
//! host.register("fetch", Box::new(ClientFetchFunction));
//! let mut vm = Vm::new(host);
//! let cont = Continuation::from_json(&received_cont_json).unwrap();
//! let result = vm.eval_resumable(&expr, &mut ctx, Some(cont));
//! ```

mod cache;
mod host;

pub use cache::{CallCache, CallKey, call_key};
pub use host::CacheHost;
