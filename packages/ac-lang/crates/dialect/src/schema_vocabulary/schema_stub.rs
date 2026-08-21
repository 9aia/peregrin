use ac_runtime::NonFunctionValue;

use crate::schema_vocabulary::Schema;
use crate::schema_vocabulary::validation_error::ValidationError;

/// A minimal schema validator for testing.
///
/// Always succeeds (accepts any value).
pub struct StubSchema;

impl Schema<NonFunctionValue> for StubSchema {
    fn validate(&self, _value: &NonFunctionValue) -> Result<(), ValidationError> {
        Ok(())
    }
}
