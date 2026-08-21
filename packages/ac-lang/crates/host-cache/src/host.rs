use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use ac_runtime::RuntimeValue;
use ac_vm::{Context, Error, Function, Host};

use crate::cache::{CallCache, CallKey};

/// A function wrapper that checks the cache before dispatching.
///
/// If the cache has a result for this call, it returns it directly.
/// Otherwise, it dispatches to the inner function and records
/// the result in the cache.
struct CachedFunction {
    inner: Box<dyn Function>,
    cache: Arc<Mutex<CallCache>>,
    name: String,
}

impl Function for CachedFunction {
    fn call(&self, args: &[RuntimeValue], ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let key = CallKey::new(&self.name, args);
        if let Some(cached) = self.cache.lock().unwrap().get_by_key(&key) {
            return Ok(cached.clone());
        }

        let result = self.inner.call(args, ctx)?;

        // Only cache successful results, not yields.
        self.cache
            .lock()
            .unwrap()
            .insert(&self.name, args, result.clone());

        Ok(result)
    }
}

/// A host that caches function call results.
///
/// Functions are registered with [`register`](Self::register). Each
/// call is checked against the [`CallCache`] before dispatching. On
/// a cache hit, the cached result is returned without calling the
/// inner function. On a miss, the inner function is called and the
/// result is recorded.
///
/// The cache is serializable via [`cache`](Self::cache) and
/// [`CallCache::to_json`], enabling migration across machines.
///
/// # Yielding
///
/// When a function returns `Err(Error::Yield(value))`, the result
/// is **not** cached. On resume, the call will be re-dispatched.
/// The caller is expected to populate the cache with the yielded
/// result before resuming (e.g. by calling
/// [`cache_mut`](Self::cache_mut).`insert(...)`).
pub struct CacheHost {
    functions: HashMap<String, CachedFunction>,
    cache: Arc<Mutex<CallCache>>,
}

impl CacheHost {
    /// Creates a new `CacheHost` with an empty cache.
    pub fn new() -> Self {
        CacheHost {
            functions: HashMap::new(),
            cache: Arc::new(Mutex::new(CallCache::new())),
        }
    }

    /// Creates a new `CacheHost` with a pre-populated cache.
    ///
    /// Use this on the client side when resuming a yielded
    /// evaluation. The cache should contain results for all calls
    /// that were completed before the yield.
    pub fn with_cache(cache: CallCache) -> Self {
        CacheHost {
            functions: HashMap::new(),
            cache: Arc::new(Mutex::new(cache)),
        }
    }

    /// Registers a function under the given name.
    ///
    /// All calls to this function will be checked against the cache
    /// before dispatching.
    pub fn register(&mut self, name: impl Into<String>, function: Box<dyn Function>) {
        let name = name.into();
        self.functions.insert(
            name.clone(),
            CachedFunction {
                inner: function,
                cache: self.cache.clone(),
                name,
            },
        );
    }

    /// Returns a reference to the call cache.
    pub fn cache(&self) -> CallCache {
        self.cache.lock().unwrap().clone()
    }

    /// Returns a mutable reference to the call cache.
    pub fn cache_mut(&self) -> std::sync::MutexGuard<'_, CallCache> {
        self.cache.lock().unwrap()
    }
}

impl Default for CacheHost {
    fn default() -> Self {
        Self::new()
    }
}

impl Host for CacheHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        self.functions.get(name).map(|f| f as &dyn Function)
    }
}

impl Clone for CacheHost {
    fn clone(&self) -> Self {
        CacheHost {
            functions: HashMap::new(),
            cache: self.cache.clone(),
        }
    }
}
