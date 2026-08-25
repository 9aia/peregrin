use std::cmp::Ordering;

use handl_vm::{Context, Function};
use handl_vm_common::Module;
use handl_vm_values_json::{Error, NonFunctionValue};
use serde_json::Value as JsonValue;

use crate::shared::{arr as to_arr, b as to_bool, num, require_array, require_one_arg};

/// The `cmp` module — comparison and ordering utilities.
///
/// Provides functions callable as `cmp.eq(a, b)`, `cmp.lt(a, b)`,
/// `cmp.min(a, b, ...)`, etc. in handl expressions.
#[derive(Clone, Copy)]
pub struct CmpModule;

impl Module for CmpModule {
    fn name(&self) -> &str {
        "cmp"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "eq" => Some(&Eq),
            "ne" => Some(&Ne),
            "lt" => Some(&Lt),
            "le" => Some(&Le),
            "gt" => Some(&Gt),
            "ge" => Some(&Ge),
            "compare" => Some(&Compare),
            "min" => Some(&Min),
            "max" => Some(&Max),
            "clamp" => Some(&Clamp),
            "between" => Some(&Between),
            "in_range" => Some(&InRange),
            "is_sorted" => Some(&IsSorted),
            "is_sorted_desc" => Some(&IsSortedDesc),
            "sort" => Some(&Sort),
            "sort_desc" => Some(&SortDesc),
            _ => None,
        }
    }
}

fn value_kind(v: &JsonValue) -> u8 {
    match v {
        JsonValue::Null => 0,
        JsonValue::Bool(_) => 1,
        JsonValue::Number(_) => 2,
        JsonValue::String(_) => 3,
        JsonValue::Array(_) => 4,
        JsonValue::Object(_) => 5,
    }
}

fn compare_values(a: &JsonValue, b: &JsonValue) -> Ordering {
    value_kind(a)
        .cmp(&value_kind(b))
        .then_with(|| match (a, b) {
            (JsonValue::Null, JsonValue::Null) => Ordering::Equal,
            (JsonValue::Bool(aa), JsonValue::Bool(bb)) => aa.cmp(bb),
            (JsonValue::Number(aa), JsonValue::Number(bb)) => {
                aa.as_f64().unwrap().total_cmp(&bb.as_f64().unwrap())
            }
            (JsonValue::String(aa), JsonValue::String(bb)) => aa.cmp(bb),
            (JsonValue::Array(aa), JsonValue::Array(bb)) => {
                for (x, y) in aa.iter().zip(bb.iter()) {
                    let c = compare_values(x, y);
                    if c != Ordering::Equal {
                        return c;
                    }
                }
                aa.len().cmp(&bb.len())
            }
            (JsonValue::Object(aa), JsonValue::Object(bb)) => {
                if aa.len() != bb.len() {
                    return aa.len().cmp(&bb.len());
                }
                let mut av: Vec<_> = aa.iter().map(|(k, v)| (k.as_str(), v)).collect();
                let mut bv: Vec<_> = bb.iter().map(|(k, v)| (k.as_str(), v)).collect();
                av.sort_by(|x, y| x.0.cmp(y.0));
                bv.sort_by(|x, y| x.0.cmp(y.0));
                for ((ak, avv), (bk, bvv)) in av.iter().zip(bv.iter()) {
                    let c = ak.cmp(bk);
                    if c != Ordering::Equal {
                        return c;
                    }
                    let c = compare_values(avv, bvv);
                    if c != Ordering::Equal {
                        return c;
                    }
                }
                Ordering::Equal
            }
            _ => Ordering::Equal,
        })
}

fn compare_nfv(a: &NonFunctionValue, b: &NonFunctionValue) -> Ordering {
    compare_values(a.inner(), b.inner())
}

fn require_two(args: &[NonFunctionValue]) -> Result<(&NonFunctionValue, &NonFunctionValue), Error> {
    if args.len() < 2 {
        return Err(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }
    Ok((&args[0], &args[1]))
}

fn require_three(
    args: &[NonFunctionValue],
) -> Result<(&NonFunctionValue, &NonFunctionValue, &NonFunctionValue), Error> {
    if args.len() < 3 {
        return Err(Error::ArgumentCount {
            expected: 3,
            got: args.len(),
        });
    }
    Ok((&args[0], &args[1], &args[2]))
}

#[derive(Clone, Copy)]
pub struct Eq;
impl Function<NonFunctionValue> for Eq {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b_arg) = require_two(args)?;
        Ok(to_bool(a.inner() == b_arg.inner()))
    }
}

#[derive(Clone, Copy)]
pub struct Ne;
impl Function<NonFunctionValue> for Ne {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b_arg) = require_two(args)?;
        Ok(to_bool(a.inner() != b_arg.inner()))
    }
}

#[derive(Clone, Copy)]
pub struct Lt;
impl Function<NonFunctionValue> for Lt {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b_arg) = require_two(args)?;
        Ok(to_bool(compare_nfv(a, b_arg) == Ordering::Less))
    }
}

#[derive(Clone, Copy)]
pub struct Le;
impl Function<NonFunctionValue> for Le {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b_arg) = require_two(args)?;
        Ok(to_bool(compare_nfv(a, b_arg) != Ordering::Greater))
    }
}

#[derive(Clone, Copy)]
pub struct Gt;
impl Function<NonFunctionValue> for Gt {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b_arg) = require_two(args)?;
        Ok(to_bool(compare_nfv(a, b_arg) == Ordering::Greater))
    }
}

#[derive(Clone, Copy)]
pub struct Ge;
impl Function<NonFunctionValue> for Ge {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b_arg) = require_two(args)?;
        Ok(to_bool(compare_nfv(a, b_arg) != Ordering::Less))
    }
}

#[derive(Clone, Copy)]
pub struct Compare;
impl Function<NonFunctionValue> for Compare {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b_arg) = require_two(args)?;
        Ok(num(match compare_nfv(a, b_arg) {
            Ordering::Less => -1.0,
            Ordering::Equal => 0.0,
            Ordering::Greater => 1.0,
        }))
    }
}

#[derive(Clone, Copy)]
pub struct Min;
impl Function<NonFunctionValue> for Min {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.is_empty() {
            return Err(Error::ArgumentCount {
                expected: 1,
                got: 0,
            });
        }
        let mut best: &NonFunctionValue = &args[0];
        for arg in args.iter().skip(1) {
            if compare_nfv(arg, best) == Ordering::Less {
                best = arg;
            }
        }
        Ok(best.clone())
    }
}

#[derive(Clone, Copy)]
pub struct Max;
impl Function<NonFunctionValue> for Max {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.is_empty() {
            return Err(Error::ArgumentCount {
                expected: 1,
                got: 0,
            });
        }
        let mut best: &NonFunctionValue = &args[0];
        for arg in args.iter().skip(1) {
            if compare_nfv(arg, best) == Ordering::Greater {
                best = arg;
            }
        }
        Ok(best.clone())
    }
}

#[derive(Clone, Copy)]
pub struct Clamp;
impl Function<NonFunctionValue> for Clamp {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (value, lo, hi) = require_three(args)?;
        if compare_nfv(lo, hi) == Ordering::Greater {
            return Err(Error::PrimitiveError(
                "clamp bounds are inverted".to_string(),
            ));
        }
        if compare_nfv(value, lo) == Ordering::Less {
            return Ok(lo.clone());
        }
        if compare_nfv(value, hi) == Ordering::Greater {
            return Ok(hi.clone());
        }
        Ok(value.clone())
    }
}

fn between_impl(args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let (x, lo, hi) = require_three(args)?;
    Ok(to_bool(
        compare_nfv(lo, x) != Ordering::Greater && compare_nfv(x, hi) != Ordering::Greater,
    ))
}

#[derive(Clone, Copy)]
pub struct Between;
impl Function<NonFunctionValue> for Between {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        between_impl(args)
    }
}

#[derive(Clone, Copy)]
pub struct InRange;
impl Function<NonFunctionValue> for InRange {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        between_impl(args)
    }
}

fn is_sorted_impl(args: &[NonFunctionValue], descending: bool) -> Result<NonFunctionValue, Error> {
    let input = require_one_arg(args)?;
    let arr_val = require_array(input)?;
    for window in arr_val.windows(2) {
        let c = compare_values(&window[0], &window[1]);
        if descending && c == Ordering::Less {
            return Ok(to_bool(false));
        }
        if !descending && c == Ordering::Greater {
            return Ok(to_bool(false));
        }
    }
    Ok(to_bool(true))
}

#[derive(Clone, Copy)]
pub struct IsSorted;
impl Function<NonFunctionValue> for IsSorted {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        is_sorted_impl(args, false)
    }
}

#[derive(Clone, Copy)]
pub struct IsSortedDesc;
impl Function<NonFunctionValue> for IsSortedDesc {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        is_sorted_impl(args, true)
    }
}

fn sort_impl(args: &[NonFunctionValue], descending: bool) -> Result<NonFunctionValue, Error> {
    let input = require_one_arg(args)?;
    let arr_val = require_array(input)?;
    let mut sorted: Vec<JsonValue> = arr_val.to_vec();
    if descending {
        sorted.sort_by(|a, c| compare_values(c, a));
    } else {
        sorted.sort_by(|a, c| compare_values(a, c));
    }
    Ok(to_arr(&sorted))
}

#[derive(Clone, Copy)]
pub struct Sort;
impl Function<NonFunctionValue> for Sort {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        sort_impl(args, false)
    }
}

#[derive(Clone, Copy)]
pub struct SortDesc;
impl Function<NonFunctionValue> for SortDesc {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        sort_impl(args, true)
    }
}
