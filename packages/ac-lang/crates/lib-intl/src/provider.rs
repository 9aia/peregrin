use std::sync::Arc;

use icu_provider_blob::BlobDataProvider;

/// A shared ICU4X data provider backed by a blob.
///
/// When set on `IntlModule`, all functions will use this provider
/// instead of the compiled CLDR data. This enables runtime data loading
/// with custom locale subsets for smaller binaries.
///
/// Create from a `.postcard` blob file:
/// ```no_run
/// use ac_lib_intl::IntlModule;
///
/// let blob = std::fs::read("data.postcard").unwrap();
/// let module = IntlModule::with_blob_provider(blob);
/// ```
///
/// The raw bytes are stored as `Arc<[u8]>` so that cloning `IntlProvider`
/// (e.g. when distributing it to each function struct in `with_provider`)
/// is a cheap refcount increment, not a deep copy.
/// A `BlobDataProvider` is created on each function call from the stored
/// bytes, which requires copying into a `Box<[u8]>` (ICU4X API requirement).
#[derive(Clone)]
pub struct IntlProvider {
    blob: Arc<[u8]>,
}

impl IntlProvider {
    /// Create a provider from a blob of ICU4X postcard data.
    pub fn from_blob(blob: Vec<u8>) -> Self {
        Self { blob: Arc::from(blob) }
    }

    /// Create a `BlobDataProvider` from the stored bytes.
    ///
    /// This requires copying the bytes into a `Box<[u8]>` (ICU4X API requirement).
    pub fn as_blob_data_provider(&self) -> Result<BlobDataProvider, icu_provider::DataError> {
        BlobDataProvider::try_new_from_blob(self.blob.as_ref().into())
    }

    /// Returns the raw blob bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.blob
    }

    /// Returns the raw pointer to the blob data, for testing that clones share allocation.
    #[doc(hidden)]
    pub fn blob_ptr(&self) -> *const u8 {
        Arc::as_ptr(&self.blob) as *const u8
    }

    /// Returns the strong reference count of the inner `Arc`, for testing.
    #[doc(hidden)]
    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.blob)
    }
}
