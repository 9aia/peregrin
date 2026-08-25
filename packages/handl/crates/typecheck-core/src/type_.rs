use std::collections::BTreeMap;

use serde_json::Value as JsonValue;

/// A static type in the handl type system.
///
/// Types are derived from schema fragments via a [`SchemaInterpreter`].
/// The typechecker compares inferred types against declared types to
/// detect mismatches before runtime evaluation.
///
/// `Unknown` is compatible with everything — it represents types that
/// cannot be statically determined. `Never` represents impossible code
/// paths (e.g., a function that always errors).
///
/// `Var` represents a type variable used in generic function signatures.
/// Outside of unification (during generic call instantiation), `Var` is
/// not compatible with concrete types. See [`crate::unify`].
///
/// `Trait` represents a named trait reference used in type parameter bounds.
/// A `Trait` is never a concrete type — it only appears in `TypeParam::bounds`
/// and is resolved against a [`crate::TraitRegistry`] during bound checking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    String,
    Number,
    Boolean,
    Null,
    Array(Box<Type>),
    Object(BTreeMap<String, Type>),
    Function(FunctionType),
    /// A type variable (e.g. `T`, `U`) used in generic function signatures.
    Var(String),
    /// A named trait reference (e.g. `Numeric`, `Comparable`) used in type
    /// parameter bounds. Resolved against a [`crate::TraitRegistry`].
    Trait(String),
    /// A type application: applying a type constructor to type arguments.
    ///
    /// Used for higher-kinded types (HKTs). The first element is the type
    /// constructor (e.g. `Var("F")` or `Array`), and the vector contains
    /// the type arguments (e.g. `[Var("T")]`).
    ///
    /// Example: `Array<String>` can be represented as
    /// `App(Box::new(Var("F")), vec![Type::String])` when `F` is a type
    /// parameter bound to `Array`.
    App(Box<Type>, Vec<Type>),
    Unknown,
    Never,
}

/// A type parameter declaration on a generic function.
///
/// Each type parameter has a name (e.g. `"T"`), an optional list of trait
/// bounds (e.g. `[Trait("Numeric")]`), and a variance annotation.
///
/// Variance defaults to [`Variance::Invariant`] — the safest default.
/// See [`Variance`] for details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParam {
    /// The type parameter name (e.g. `"T"`, `"U"`).
    pub name: String,
    /// Trait bounds that the concrete type must satisfy (e.g. `[Trait("Numeric")]`).
    /// Empty means no constraints — any type is accepted.
    pub bounds: Vec<Type>,
    /// Variance annotation for subtyping. Defaults to [`Variance::Invariant`].
    pub variance: Variance,
}

impl TypeParam {
    /// Create a type parameter with no bounds and invariant variance.
    pub fn new(name: impl Into<String>) -> Self {
        TypeParam {
            name: name.into(),
            bounds: Vec::new(),
            variance: Variance::Invariant,
        }
    }

    /// Create a type parameter with the given trait bounds.
    pub fn with_bounds(name: impl Into<String>, bounds: Vec<Type>) -> Self {
        TypeParam {
            name: name.into(),
            bounds,
            variance: Variance::Invariant,
        }
    }

    /// Set the variance of this type parameter.
    pub fn with_variance(mut self, variance: Variance) -> Self {
        self.variance = variance;
        self
    }
}

/// Variance annotations for type parameters.
///
/// Controls how subtyping flows through a type parameter's positions:
/// - **Covariant** (`out`): `Array<out T>` — `Array<String>` is a subtype of
///   `Array<Unknown>` if `String` is a subtype of `Unknown`.
/// - **Contravariant** (`in`): `fn(in T) -> U` — a function accepting
///   `Unknown` is usable where one accepting `String` is expected.
/// - **Invariant** (default): `Vec<T>` — no subtyping. The concrete type must
///   match exactly.
/// - **Bivariant**: compatible in both directions — rarely useful, included
///   for completeness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variance {
    Covariant,
    Contravariant,
    Invariant,
    Bivariant,
}

/// The kind of a type — tracks the "type of types" for higher-kinded types.
///
/// - [`Kind::Type`]: a concrete type (e.g. `String`, `Number`, `Array<String>`).
/// - [`Kind::Constructor`]: a type constructor that takes arguments.
///   The `Vec<Kind>` describes the argument kinds, and the boxed `Kind`
///   describes the result kind.
///
/// Examples:
/// - `String` has kind `Type`
/// - `Array` has kind `Type -> Type` (takes one type arg, produces a type)
/// - `Map` has kind `Type -> Type -> Type`
/// - `F` (a higher-kinded type variable) has kind `Type -> Type`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Type,
    Constructor(Vec<Kind>, Box<Kind>),
}

impl Kind {
    /// The kind of a proper type (no arguments).
    pub fn type_kind() -> Self {
        Kind::Type
    }

    /// Construct a `Type -> Type` kind (unary type constructor).
    pub fn unary() -> Self {
        Kind::Constructor(vec![Kind::Type], Box::new(Kind::Type))
    }

    /// Construct a `Type -> Type -> Type` kind (binary type constructor).
    pub fn binary() -> Self {
        Kind::Constructor(vec![Kind::Type, Kind::Type], Box::new(Kind::Type))
    }
}

/// A function type — type parameters, parameter names/types, and return type.
///
/// `type_params` is empty for monomorphic functions. When non-empty, the
/// function is generic and the typechecker unifies argument types against
/// parameter types to instantiate the return type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionType {
    /// Type parameters (e.g. `[TypeParam::new("T")]`). Empty for monomorphic functions.
    pub type_params: Vec<TypeParam>,
    /// `(param_name, param_type)` pairs, in declaration order.
    pub params: Vec<(String, Type)>,
    /// The return type.
    pub return_type: Box<Type>,
}

impl Type {
    /// Human-readable name for error messages.
    pub fn name(&self) -> String {
        match self {
            Type::String => "string".to_string(),
            Type::Number => "number".to_string(),
            Type::Boolean => "boolean".to_string(),
            Type::Null => "null".to_string(),
            Type::Array(item) => format!("array<{}>", item.name()),
            Type::Object(props) => {
                let inner = props
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", v.name()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("object{{{inner}}}")
            }
            Type::Function(f) => {
                let params = f
                    .params
                    .iter()
                    .map(|(n, t)| format!("{n}: {}", t.name()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("fn({params}) -> {}", f.return_type.name())
            }
            Type::Var(name) => name.clone(),
            Type::Trait(name) => format!("{name} (trait)"),
            Type::App(ctor, args) => {
                let args_str = args.iter().map(|a| a.name()).collect::<Vec<_>>().join(", ");
                format!("{}<{args_str}>", ctor.name())
            }
            Type::Unknown => "unknown".to_string(),
            Type::Never => "never".to_string(),
        }
    }

    /// Returns `true` if this type is `Unknown`.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Type::Unknown)
    }

    /// Encode this type as a JSON value.
    ///
    /// The encoding is a tagged union: `{"kind": "string"}`,
    /// `{"kind": "array", "item": <type>}`, etc.
    pub fn to_json(&self) -> JsonValue {
        match self {
            Type::String => serde_json::json!({"kind": "string"}),
            Type::Number => serde_json::json!({"kind": "number"}),
            Type::Boolean => serde_json::json!({"kind": "boolean"}),
            Type::Null => serde_json::json!({"kind": "null"}),
            Type::Unknown => serde_json::json!({"kind": "unknown"}),
            Type::Never => serde_json::json!({"kind": "never"}),
            Type::Var(name) => serde_json::json!({"kind": "var", "name": name}),
            Type::Trait(name) => serde_json::json!({"kind": "trait", "name": name}),
            Type::Array(item) => serde_json::json!({"kind": "array", "item": item.to_json()}),
            Type::Object(props) => {
                let mut map = serde_json::Map::new();
                for (k, v) in props {
                    map.insert(k.clone(), v.to_json());
                }
                serde_json::json!({"kind": "object", "properties": JsonValue::Object(map)})
            }
            Type::Function(f) => {
                let params: Vec<JsonValue> = f
                    .params
                    .iter()
                    .map(|(name, ty)| serde_json::json!([name, ty.to_json()]))
                    .collect();
                serde_json::json!({
                    "kind": "function",
                    "params": params,
                    "return": f.return_type.to_json()
                })
            }
            Type::App(ctor, args) => {
                let args_json: Vec<JsonValue> = args.iter().map(|a| a.to_json()).collect();
                serde_json::json!({
                    "kind": "app",
                    "constructor": ctor.to_json(),
                    "args": args_json
                })
            }
        }
    }

    /// Decode a type from a JSON value produced by [`to_json`].
    pub fn from_json(value: &JsonValue) -> Option<Self> {
        let obj = value.as_object()?;
        let kind = obj.get("kind")?.as_str()?;
        match kind {
            "string" => Some(Type::String),
            "number" => Some(Type::Number),
            "boolean" => Some(Type::Boolean),
            "null" => Some(Type::Null),
            "unknown" => Some(Type::Unknown),
            "never" => Some(Type::Never),
            "var" => Some(Type::Var(obj.get("name")?.as_str()?.to_string())),
            "trait" => Some(Type::Trait(obj.get("name")?.as_str()?.to_string())),
            "array" => Some(Type::Array(Box::new(Type::from_json(obj.get("item")?)?))),
            "object" => {
                let props = obj.get("properties")?.as_object()?;
                let mut map = BTreeMap::new();
                for (k, v) in props {
                    map.insert(k.clone(), Type::from_json(v)?);
                }
                Some(Type::Object(map))
            }
            "function" => {
                let params = obj.get("params")?.as_array()?;
                let mut param_list = Vec::new();
                for p in params {
                    let arr = p.as_array()?;
                    let name = arr.get(0)?.as_str()?.to_string();
                    let ty = Type::from_json(arr.get(1)?)?;
                    param_list.push((name, ty));
                }
                let return_type = Box::new(Type::from_json(obj.get("return")?)?);
                Some(Type::Function(FunctionType {
                    type_params: Vec::new(),
                    params: param_list,
                    return_type,
                }))
            }
            "app" => {
                let ctor = Box::new(Type::from_json(obj.get("constructor")?)?);
                let args = obj.get("args")?.as_array()?;
                let mut arg_list = Vec::new();
                for a in args {
                    arg_list.push(Type::from_json(a)?);
                }
                Some(Type::App(ctor, arg_list))
            }
            _ => None,
        }
    }
}
