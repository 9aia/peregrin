//! VM host functions and the composite `TypeHost` for type checking.

mod functions;
mod type_host;

pub use functions::{
    AppArgsCompatible, FuncParamsCompatible, ObjEvery, PairSecond, TypeKind, TypeName, ZipAll,
};
pub use type_host::{TypeHelpersModule, TypeHost};
