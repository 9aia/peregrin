use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use ac_runtime::NonFunctionValue;
use ac_runtime::{
    AsyncFunction, Context, Error, Function, GenericFunction, Host, LazyFunction, Operation,
};

use super::call_cache::{CallCache, CallKey};

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
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
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

/// A decorator [`Host`] that caches function call results.
///
/// Wraps an inner host. Functions registered with
/// [`register`](Self::register) are checked against the
/// [`CallCache`] before dispatching — on a cache hit, the cached
/// result is returned without calling the inner function. On a miss,
/// the inner function is called and the result is recorded.
///
/// Function names not registered via `register` delegate to the
/// inner host without caching. This lets you compose hosts while
/// selectively caching only the functions that need it.
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
///
/// # Example: composition with `DefaultHost`
///
/// ```ignore
/// use ac_runtime_example_1::DefaultHost;
/// use ac_runtime_example_1_cache::CacheHost;
///
/// let host = CacheHost::new(DefaultHost::default());
/// // add, mul, etc. delegate to DefaultHost (uncached)
/// // register expensive functions for caching:
/// // host.register("fetch", Box::new(FetchFunction));
/// ```
pub struct CacheHost<H: Host> {
    inner: H,
    registered: HashMap<String, CachedFunction>,
    cache: Arc<Mutex<CallCache>>,
}

impl<H: Host> CacheHost<H> {
    /// Creates a new `CacheHost` wrapping `inner` with an empty cache.
    pub fn new(inner: H) -> Self {
        CacheHost {
            inner,
            registered: HashMap::new(),
            cache: Arc::new(Mutex::new(CallCache::new())),
        }
    }

    /// Creates a new `CacheHost` wrapping `inner` with a pre-populated
    /// cache.
    ///
    /// Use this on the client side when resuming a yielded
    /// evaluation. The cache should contain results for all calls
    /// that were completed before the yield.
    pub fn with_cache(inner: H, cache: CallCache) -> Self {
        CacheHost {
            inner,
            registered: HashMap::new(),
            cache: Arc::new(Mutex::new(cache)),
        }
    }

    /// Registers a function under the given name for caching.
    ///
    /// All calls to this function will be checked against the cache
    /// before dispatching. Function names not registered delegate to
    /// the inner host without caching.
    pub fn register(&mut self, name: impl Into<String>, function: Box<dyn Function>) {
        let name = name.into();
        self.registered.insert(
            name.clone(),
            CachedFunction {
                inner: function,
                cache: self.cache.clone(),
                name,
            },
        );
    }

    /// Returns a reference to the inner host.
    pub fn inner(&self) -> &H {
        &self.inner
    }

    /// Returns a clone of the call cache.
    pub fn cache(&self) -> CallCache {
        self.cache.lock().unwrap().clone()
    }

    /// Returns a mutable reference to the call cache.
    pub fn cache_mut(&self) -> std::sync::MutexGuard<'_, CallCache> {
        self.cache.lock().unwrap()
    }
}

impl<H: Host + Default> Default for CacheHost<H> {
    fn default() -> Self {
        Self::new(H::default())
    }
}

impl<H: Host> Host for CacheHost<H> {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        if let Some(f) = self.registered.get(name) {
            return Some(f as &dyn Function);
        }
        self.inner.function(name)
    }

    fn async_function(&self, name: &str) -> Option<&dyn AsyncFunction> {
        self.inner.async_function(name)
    }

    fn operation(&self, name: &str) -> Option<&dyn Operation> {
        self.inner.operation(name)
    }

    fn lazy_function(&self, name: &str) -> Option<&dyn LazyFunction> {
        self.inner.lazy_function(name)
    }

    fn generic_function(&self, name: &str) -> Option<&dyn GenericFunction> {
        self.inner.generic_function(name)
    }
}

impl<H: Host + Clone> Clone for CacheHost<H> {
    fn clone(&self) -> Self {
        CacheHost {
            inner: self.inner.clone(),
            registered: HashMap::new(),
            cache: self.cache.clone(),
        }
    }
}
