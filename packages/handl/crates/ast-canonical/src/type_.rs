use handl_ast_common::TypeFactory;

/// A type in the handl language.
///
/// The canonical type AST supports primitives, arrays, objects, and
/// function types. More advanced forms (generics, type variables, unions)
/// can be added behind new variants as the type system grows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// A named primitive type such as `string`, `number`, or `boolean`.
    Primitive(String),

    /// An array whose elements are all of the contained type.
    Array(Box<Type>),

    /// An object with a fixed set of named fields.
    Object(Vec<(String, Type)>),

    /// A function from parameter types to a return type.
    Function { params: Vec<Type>, ret: Box<Type> },
}

impl Type {
    /// Creates a primitive type.
    pub fn primitive(name: impl Into<String>) -> Self {
        Type::Primitive(name.into())
    }

    /// Creates an array type.
    pub fn array(element: Type) -> Self {
        Type::Array(Box::new(element))
    }

    /// Creates an object type.
    pub fn object(fields: Vec<(String, Type)>) -> Self {
        Type::Object(fields)
    }

    /// Creates a function type.
    pub fn function(params: Vec<Type>, ret: Type) -> Self {
        Type::Function {
            params,
            ret: Box::new(ret),
        }
    }
}

/// The canonical handl type factory.
#[derive(Debug, Default, Clone, Copy)]
pub struct CanonicalTypeFactory;

impl TypeFactory<Type> for CanonicalTypeFactory {
    fn primitive<S>(&self, name: S) -> Type
    where
        S: Into<String>,
    {
        Type::primitive(name)
    }

    fn array(&self, element: Type) -> Type {
        Type::array(element)
    }

    fn object(&self, fields: Vec<(String, Type)>) -> Type {
        Type::object(fields)
    }

    fn function(&self, params: Vec<Type>, ret: Type) -> Type {
        Type::function(params, ret)
    }
}
