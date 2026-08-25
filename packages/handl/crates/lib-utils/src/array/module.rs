use handl_vm::{Function, GenericFunction};
use handl_vm_values_json::NonFunctionValue;

use handl_vm_common::Module;

use super::basic_operations::{
    At, Concat, Contains, DeepFlatten, Fill, First, Flatten, IndexOf, Last, Length, Pop, Push,
    Reverse, Shift, Slice, Sort,
};
use super::higher_order::{
    Chunk, Compact, CountBy, Every, Filter, Find, FlatMap, GroupBy, Map, MaxBy, MinBy, Partition,
    Pluck, Range, Reduce, SomeFn, SortBy, SortByDesc, Zip,
};
use super::iteration::{
    Dedupe, Drop, DropWhile, Enumerate, FindIndex, FindLast, FindLastIndex, Foldr, InsertAt,
    Intercalate, Intersperse, Prepend, RemoveAt, Scan, Take, TakeWhile, Transpose, Unzip, Windows,
};
use super::numeric_aggregates::{Avg, Max, Mean, Median, Min, Product, Stddev, Sum};

/// The `array` module — array utility functions.
///
/// Provides functions callable as `array.length(a)`, `array.reverse(a)`,
/// etc. in handl expressions.
#[derive(Clone, Copy)]
pub struct ArrayModule;

impl Module for ArrayModule {
    fn name(&self) -> &str {
        "array"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            // basic_operations
            "length" => Some(&Length),
            "reverse" => Some(&Reverse),
            "concat" => Some(&Concat),
            "contains" => Some(&Contains),
            "index_of" => Some(&IndexOf),
            "slice" => Some(&Slice),
            "first" => Some(&First),
            "last" => Some(&Last),
            "sort" => Some(&Sort),
            "flatten" => Some(&Flatten),
            "deep_flatten" => Some(&DeepFlatten),
            "at" => Some(&At),
            "push" => Some(&Push),
            "fill" => Some(&Fill),
            "pop" => Some(&Pop),
            "shift" => Some(&Shift),
            // numeric_aggregates
            "sum" => Some(&Sum),
            "avg" => Some(&Avg),
            "min" => Some(&Min),
            "max" => Some(&Max),
            "product" => Some(&Product),
            "mean" => Some(&Mean),
            "median" => Some(&Median),
            "stddev" => Some(&Stddev),
            // iteration
            "take" => Some(&Take),
            "drop" => Some(&Drop),
            "enumerate" => Some(&Enumerate),
            "unzip" => Some(&Unzip),
            "intersperse" => Some(&Intersperse),
            "intercalate" => Some(&Intercalate),
            "dedupe" => Some(&Dedupe),
            "prepend" => Some(&Prepend),
            "take_while" => Some(&TakeWhile),
            "drop_while" => Some(&DropWhile),
            "scan" => Some(&Scan),
            "foldr" => Some(&Foldr),
            "find_index" => Some(&FindIndex),
            "find_last" => Some(&FindLast),
            "find_last_index" => Some(&FindLastIndex),
            "windows" => Some(&Windows),
            "transpose" => Some(&Transpose),
            "insert_at" => Some(&InsertAt),
            "remove_at" => Some(&RemoveAt),
            // higher_order
            "range" => Some(&Range),
            "zip" => Some(&Zip),
            "chunk" => Some(&Chunk),
            "map" => Some(&Map),
            "filter" => Some(&Filter),
            "reduce" => Some(&Reduce),
            "flat_map" => Some(&FlatMap),
            "find" => Some(&Find),
            "some" => Some(&SomeFn),
            "every" => Some(&Every),
            "group_by" => Some(&GroupBy),
            "partition" => Some(&Partition),
            "sort_by" => Some(&SortBy),
            "sort_by_desc" => Some(&SortByDesc),
            "compact" => Some(&Compact),
            "pluck" => Some(&Pluck),
            "min_by" => Some(&MinBy),
            "max_by" => Some(&MaxBy),
            "count_by" => Some(&CountBy),
            _ => None,
        }
    }

    fn generic_function(&self, name: &str) -> Option<&dyn GenericFunction<NonFunctionValue>> {
        match name {
            "map" => Some(&Map),
            "filter" => Some(&Filter),
            "reduce" => Some(&Reduce),
            "flat_map" => Some(&FlatMap),
            "find" => Some(&Find),
            "some" => Some(&SomeFn),
            "every" => Some(&Every),
            "group_by" => Some(&GroupBy),
            "partition" => Some(&Partition),
            "sort_by" => Some(&SortBy),
            "sort_by_desc" => Some(&SortByDesc),
            "compact" => Some(&Compact),
            "pluck" => Some(&Pluck),
            "min_by" => Some(&MinBy),
            "max_by" => Some(&MaxBy),
            "count_by" => Some(&CountBy),
            _ => None,
        }
    }
}
