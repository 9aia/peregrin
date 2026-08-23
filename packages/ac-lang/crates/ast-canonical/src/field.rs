/// A named binding of a value.
///
/// A field associates a name with a value. Fields are the building
/// blocks of objects and configurations — an object is conceptually
/// a collection of fields.
///
/// `Field` is generic over the value type `V`, keeping `ac-ast`
/// dependency-free.
#[derive(Debug, Clone, PartialEq)]
pub struct Field<V> {
    /// The field name.
    pub name: String,

    /// The field's value.
    pub value: V,
}

impl<V> Field<V> {
    /// Creates a new field.
    pub fn new(name: impl Into<String>, value: V) -> Self {
        Field {
            name: name.into(),
            value,
        }
    }
}
