use ac_runtime::RuntimeValue;

use crate::schema_vocabulary::Schema;
use crate::schema_vocabulary::schema_stub::StubSchema;

#[test]
fn stub_schema_validates_any_value() {
    let schema = StubSchema;
    let value = RuntimeValue::new(serde_json::Value::Null);
    assert!(schema.validate(&value).is_ok());
}
