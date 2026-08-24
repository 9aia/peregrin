use ac_lib_os::{Env, OsError};

/// Environment variable implementation backed by [`std::env`].
#[derive(Default)]
pub struct StdEnv;

impl Env for StdEnv {
    fn get(&self, key: &str) -> Result<Option<String>, OsError> {
        Ok(std::env::var(key).ok())
    }

    fn has(&self, key: &str) -> Result<bool, OsError> {
        Ok(std::env::var(key).is_ok())
    }

    fn keys(&self) -> Result<Vec<String>, OsError> {
        Ok(std::env::vars().map(|(k, _)| k).collect())
    }
}