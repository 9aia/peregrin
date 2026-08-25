//! In-memory [`Env`] stub for testing.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::env::Env;
use crate::error::OsError;

/// A simple in-memory environment for tests.
#[derive(Default)]
pub struct StubEnv {
    pub(crate) vars: Mutex<HashMap<String, String>>,
}

impl StubEnv {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(key: impl Into<String>, value: impl Into<String>) -> Self {
        let mut vars = HashMap::new();
        vars.insert(key.into(), value.into());
        StubEnv {
            vars: Mutex::new(vars),
        }
    }
}

impl Env for StubEnv {
    fn get(&self, key: &str) -> Result<Option<String>, OsError> {
        Ok(self.vars.lock().unwrap().get(key).cloned())
    }

    fn has(&self, key: &str) -> Result<bool, OsError> {
        Ok(self.vars.lock().unwrap().contains_key(key))
    }

    fn keys(&self) -> Result<Vec<String>, OsError> {
        Ok(self.vars.lock().unwrap().keys().cloned().collect())
    }
}
