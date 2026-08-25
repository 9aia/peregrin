use handl_vm::{Function, Host};
use handl_vm_host_canonical::CanonicalHost;
use handl_vm_values_json::NonFunctionValue;

use crate::functions::{
    AstArgs, AstIsCall, AstName, AstToString, Eval, IsFunction, Keys, Parse, TypeOf,
};

/// A host that provides all [`CanonicalHost`] functions plus meta
/// functions for reflection, dynamic evaluation, and AST manipulation.
///
/// Meta functions:
/// - `typeof(value)` → type name string
/// - `keys(object)` → array of key names
/// - `is_function(value)` → boolean
/// - `eval(expr_string)` → evaluated value
/// - `parse(expr_string)` → JSON AST
/// - `ast_to_string(ast)` → expression string
/// - `ast_name(ast)` → function name or null
/// - `ast_args(ast)` → array of arg ASTs
/// - `ast_is_call(ast)` → boolean
#[derive(Clone)]
pub struct MetaHost {
    default: CanonicalHost,
    type_of: TypeOf,
    keys: Keys,
    is_function: IsFunction,
    eval: Eval,
    parse: Parse,
    ast_to_string: AstToString,
    ast_name: AstName,
    ast_args: AstArgs,
    ast_is_call: AstIsCall,
}

impl Default for MetaHost {
    fn default() -> Self {
        MetaHost {
            default: CanonicalHost::default(),
            type_of: TypeOf,
            keys: Keys,
            is_function: IsFunction,
            eval: Eval,
            parse: Parse,
            ast_to_string: AstToString,
            ast_name: AstName,
            ast_args: AstArgs,
            ast_is_call: AstIsCall,
        }
    }
}

impl Host<NonFunctionValue> for MetaHost {
    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "typeof" => Some(&self.type_of),
            "keys" => Some(&self.keys),
            "is_function" => Some(&self.is_function),
            "eval" => Some(&self.eval),
            "parse" => Some(&self.parse),
            "ast_to_string" => Some(&self.ast_to_string),
            "ast_name" => Some(&self.ast_name),
            "ast_args" => Some(&self.ast_args),
            "ast_is_call" => Some(&self.ast_is_call),
            _ => self.default.function(name),
        }
    }
}
