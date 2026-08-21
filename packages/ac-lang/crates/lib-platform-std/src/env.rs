use ac_lib_platform::{Env, PlatformError};

/// Environment variable implementation backed by [`std::env`].
#[derive(Default)]
pub struct StdEnv;

impl Env for StdEnv {
    fn get(&self, key: &str) -> Result<Option<String>, PlatformError> {
        Ok(std::env::var(key).ok())
    }

    fn has(&self, key: &str) -> Result<bool, PlatformError> {
        Ok(std::env::var(key).is_ok())
    }

    fn keys(&self) -> Result<Vec<String>, PlatformError> {
        Ok(std::env::vars().map(|(k, _)| k).collect())
    }
}
