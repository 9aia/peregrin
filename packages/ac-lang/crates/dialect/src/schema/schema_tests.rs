use ac_runtime::RuntimeValue;

use crate::schema::Schema;
use crate::schema::schema_stub::StubSchema;

#[test]
fn stub_schema_validates_any_value() {
    let schema = StubSchema;
    let value = RuntimeValue::new(serde_json::Value::Null);
    assert!(schema.validate(&value).is_ok());
}
