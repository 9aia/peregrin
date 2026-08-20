use super::*;

#[test]
fn new_field() {
    let field = Field::new("name", "ac");
    assert_eq!(field.name, "name");
    assert_eq!(field.value, "ac");
}

#[test]
fn field_debug_and_clone() {
    let field = Field::new("version", 1);
    let cloned = field.clone();
    assert_eq!(field, cloned);
}
