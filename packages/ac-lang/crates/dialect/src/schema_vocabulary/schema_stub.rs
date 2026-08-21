use ac_runtime::RuntimeValue;

use crate::schema_vocabulary::Schema;
use crate::schema_vocabulary::validation_error::ValidationError;

/// A minimal schema validator for testing.
///
/// Always succeeds (accepts any value).
pub struct StubSchema;

impl Schema<RuntimeValue> for StubSchema {
    fn validate(&self, _value: &RuntimeValue) -> Result<(), ValidationError> {
        Ok(())
    }
}
