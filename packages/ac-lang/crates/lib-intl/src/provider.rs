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
/// The raw bytes are stored (not the `BlobDataProvider` itself) because
/// `BlobDataProvider` uses `Rc` internally and is not `Send + Sync`.
/// A `BlobDataProvider` is created on each function call from the stored
/// bytes, which is a cheap zero-copy operation.
#[derive(Clone)]
pub struct IntlProvider {
    blob: Vec<u8>,
}

impl IntlProvider {
    /// Create a provider from a blob of ICU4X postcard data.
    pub fn from_blob(blob: Vec<u8>) -> Self {
        Self { blob }
    }

    /// Create a `BlobDataProvider` from the stored bytes.
    ///
    /// This is a cheap zero-copy operation that parses the blob header.
    pub fn as_blob_data_provider(&self) -> Result<BlobDataProvider, icu_provider::DataError> {
        BlobDataProvider::try_new_from_blob(self.blob.clone().into())
    }

    /// Returns the raw blob bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.blob
    }
}
