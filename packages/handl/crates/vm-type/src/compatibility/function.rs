//! `is_compatible` as an handl `FunctionValue`.

use handl_vm_values_json::FunctionValue;

use serde_json::Value as JsonValue;

/// The handl `is_compatible` function as a `FunctionValue`.
///
/// The body is an expression string that uses VM primitives to
/// pattern-match on JSON-encoded types. It uses `bool.if` for
/// lazy short-circuit evaluation, preventing infinite recursion.
pub fn is_compatible_function_value() -> FunctionValue {
    FunctionValue {
        params: vec![
            ("actual".to_string(), JsonValue::Null),
            ("expected".to_string(), JsonValue::Null),
        ],
        return_schema: Some(JsonValue::Bool(true)),
        body: IS_COMPATIBLE_BODY.to_string(),
        patterns: Vec::new(),
    }
}

/// The expression body for `is_compatible`.
///
/// Structure:
/// 1. Wildcard check: Unknown/Never → true
/// 2. Kind-based dispatch via bool.if chain
/// 3. Recursive calls via `apply(is_compatible, [a, e])`
/// 4. Helper host functions for iteration patterns
pub(crate) const IS_COMPATIBLE_BODY: &str = "\
bool.if(bool.or(str.equals(type.kind(actual), \"unknown\"), str.equals(type.kind(expected), \"unknown\")), \
true, \
bool.if(bool.or(str.equals(type.kind(actual), \"never\"), str.equals(type.kind(expected), \"never\")), \
true, \
bool.if(str.equals(type.kind(actual), \"var\"), \
bool.if(str.equals(type.kind(expected), \"var\"), str.equals(type.name(actual), type.name(expected)), false), \
bool.if(str.equals(type.kind(actual), \"trait\"), \
bool.if(str.equals(type.kind(expected), \"trait\"), str.equals(type.name(actual), type.name(expected)), false), \
bool.if(str.equals(type.kind(actual), \"app\"), \
bool.if(str.equals(type.kind(expected), \"app\"), \
bool.and(apply(is_compatible, [obj.get(actual, \"constructor\"), obj.get(expected, \"constructor\")]), \
type.app_args_compatible(obj.get(actual, \"args\"), obj.get(expected, \"args\"), is_compatible)), \
false), \
bool.if(str.equals(type.kind(actual), \"string\"), \
str.equals(type.kind(expected), \"string\"), \
bool.if(str.equals(type.kind(actual), \"number\"), \
str.equals(type.kind(expected), \"number\"), \
bool.if(str.equals(type.kind(actual), \"boolean\"), \
str.equals(type.kind(expected), \"boolean\"), \
bool.if(str.equals(type.kind(actual), \"null\"), \
str.equals(type.kind(expected), \"null\"), \
bool.if(str.equals(type.kind(actual), \"array\"), \
bool.if(str.equals(type.kind(expected), \"array\"), \
apply(is_compatible, [obj.get(actual, \"item\"), obj.get(expected, \"item\")]), \
false), \
bool.if(str.equals(type.kind(actual), \"object\"), \
bool.if(str.equals(type.kind(expected), \"object\"), \
type.obj_every(obj.get(actual, \"properties\"), obj.get(expected, \"properties\"), is_compatible), \
false), \
bool.if(str.equals(type.kind(actual), \"function\"), \
bool.if(str.equals(type.kind(expected), \"function\"), \
bool.and(type.func_params_compatible(obj.get(actual, \"params\"), obj.get(expected, \"params\"), is_compatible), \
apply(is_compatible, [obj.get(actual, \"return\"), obj.get(expected, \"return\")])), \
false), \
false))))))))))))";
