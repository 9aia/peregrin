use ac_runtime_values::NonFunctionValue;
use ac_runtime::Function;
use ac_runtime_modules::Module;

use super::access::{DeepGet, Get, GetOr, HasKey, PathOr};
use super::combination::{DeepMerge, FromEntries, Merge};
use super::inspection::{Entries, Keys, Size, Values};
use super::mutation::{Set, SetPath, Update, UpdatePath};
use super::selection::{Omit, Pick};
use super::transformation::{CloneDeep, FilterKeys, FilterValues, Invert, MapKeys, MapValues};

/// The `obj` module — object utility functions.
///
/// Provides functions callable as `obj.keys(o)`, `obj.get(o, k)`,
/// `obj.set(o, k, v)`, `obj.has_key(o, k)`, `obj.map_values(fn, o)`,
/// `obj.deep_get(o, path)`, `obj.deep_merge(a, b)`, etc. in AC expressions.
#[derive(Clone, Copy)]
pub struct ObjModule;

impl Module for ObjModule {
    fn name(&self) -> &str {
        "obj"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "keys" => Some(&Keys),
            "values" => Some(&Values),
            "entries" => Some(&Entries),
            "from_entries" => Some(&FromEntries),
            "pick" => Some(&Pick),
            "omit" => Some(&Omit),
            "merge" => Some(&Merge),
            "deep_get" => Some(&DeepGet),
            "deep_merge" => Some(&DeepMerge),
            "get" => Some(&Get),
            "get_or" => Some(&GetOr),
            "set" => Some(&Set),
            "update" => Some(&Update),
            "has_key" => Some(&HasKey),
            "map_values" => Some(&MapValues),
            "map_keys" => Some(&MapKeys),
            "filter_keys" => Some(&FilterKeys),
            "filter_values" => Some(&FilterValues),
            "invert" => Some(&Invert),
            "path_or" => Some(&PathOr),
            "set_path" => Some(&SetPath),
            "update_path" => Some(&UpdatePath),
            "size" => Some(&Size),
            "clone_deep" => Some(&CloneDeep),
            _ => None,
        }
    }
}
