use ac_runtime::{Context, Function};
use ac_runtime_modules::Module;
use ac_runtime_values::Error;
use ac_runtime_values::NonFunctionValue;
use base64::{Engine as _, engine::general_purpose};

use super::common::{
    b, byte_array, num, require_byte_value, require_bytes, require_index, require_one_bytes,
    require_two_bytes, s,
};

/// The `bytes` module — byte array utility functions.
#[derive(Clone, Copy)]
pub struct BytesModule;

impl Module for BytesModule {
    fn name(&self) -> &str {
        "bytes"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "length" => Some(&Length),
            "is_empty" => Some(&IsEmpty),
            "get" => Some(&Get),
            "set" => Some(&Set),
            "slice" => Some(&Slice),
            "concat" => Some(&Concat),
            "reverse" => Some(&Reverse),
            "index_of" => Some(&IndexOf),
            "find" => Some(&Find),
            "contains" => Some(&Contains),
            "starts_with" => Some(&StartsWith),
            "ends_with" => Some(&EndsWith),
            "equals" => Some(&Equals),
            "count" => Some(&Count),
            "sum" => Some(&Sum),
            "min" => Some(&Min),
            "max" => Some(&Max),
            "avg" => Some(&Avg),
            "sort" => Some(&Sort),
            "from_string" => Some(&FromString),
            "to_string" => Some(&ToString),
            "encode_hex" => Some(&EncodeHex),
            "decode_hex" => Some(&DecodeHex),
            "encode_base64" => Some(&EncodeBase64),
            "decode_base64" => Some(&DecodeBase64),
            _ => None,
        }
    }
}

fn find_sub(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if haystack.len() < needle.len() {
        return None;
    }
    for i in 0..=haystack.len() - needle.len() {
        if &haystack[i..i + needle.len()] == &needle[..] {
            return Some(i);
        }
    }
    None
}

#[derive(Clone, Copy)]
pub struct Length;
impl Function<NonFunctionValue> for Length {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let bytes = require_one_bytes(args)?;
        Ok(num(bytes.len() as f64))
    }
}

#[derive(Clone, Copy)]
pub struct IsEmpty;
impl Function<NonFunctionValue> for IsEmpty {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let bytes = require_one_bytes(args)?;
        Ok(b(bytes.is_empty()))
    }
}

#[derive(Clone, Copy)]
pub struct Get;
impl Function<NonFunctionValue> for Get {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let bytes = require_bytes(&args[0])?;
        let index = require_index(args, 1)?;
        match bytes.get(index) {
            Some(v) => Ok(num(f64::from(*v))),
            None => Ok(NonFunctionValue::new(serde_json::Value::Null)),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Set;
impl Function<NonFunctionValue> for Set {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let mut bytes = require_bytes(&args[0])?;
        let index = require_index(args, 1)?;
        let value = require_byte_value(args, 2)?;
        if index >= bytes.len() {
            return Err(Error::PrimitiveError(format!(
                "index {index} out of bounds (length: {})",
                bytes.len()
            )));
        }
        bytes[index] = value;
        Ok(byte_array(&bytes))
    }
}

#[derive(Clone, Copy)]
pub struct Slice;
impl Function<NonFunctionValue> for Slice {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let bytes = require_bytes(&args[0])?;
        let start = require_index(args, 1)?;
        let end = require_index(args, 2)?;
        let end = end.min(bytes.len());
        let start = start.min(end);
        Ok(byte_array(&bytes[start..end]))
    }
}

#[derive(Clone, Copy)]
pub struct Concat;
impl Function<NonFunctionValue> for Concat {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let mut result = Vec::new();
        for arg in args {
            result.extend(require_bytes(arg)?);
        }
        Ok(byte_array(&result))
    }
}

#[derive(Clone, Copy)]
pub struct Reverse;
impl Function<NonFunctionValue> for Reverse {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let bytes = require_one_bytes(args)?;
        let reversed: Vec<u8> = bytes.iter().rev().copied().collect();
        Ok(byte_array(&reversed))
    }
}

#[derive(Clone, Copy)]
pub struct IndexOf;
impl Function<NonFunctionValue> for IndexOf {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let haystack = require_bytes(&args[0])?;
        let needle = require_byte_value(args, 1)?;
        let idx = haystack
            .iter()
            .position(|&b| b == needle)
            .map(|i| i as f64)
            .unwrap_or(-1.0);
        Ok(num(idx))
    }
}

#[derive(Clone, Copy)]
pub struct Find;
impl Function<NonFunctionValue> for Find {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (haystack, needle) = require_two_bytes(args)?;
        let idx = find_sub(&haystack, &needle)
            .map(|i| i as f64)
            .unwrap_or(-1.0);
        Ok(num(idx))
    }
}

#[derive(Clone, Copy)]
pub struct Contains;
impl Function<NonFunctionValue> for Contains {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let haystack = require_bytes(&args[0])?;
        let needle = if let Some(n) = args[1].as_number() {
            if !n.is_finite() || n < 0.0 || n > 255.0 {
                return Err(Error::PrimitiveError(format!(
                    "byte value is not in the range 0-255: {n}"
                )));
            }
            vec![n as u8]
        } else {
            require_bytes(&args[1])?
        };
        Ok(b(find_sub(&haystack, &needle).is_some()))
    }
}

#[derive(Clone, Copy)]
pub struct StartsWith;
impl Function<NonFunctionValue> for StartsWith {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (haystack, needle) = require_two_bytes(args)?;
        Ok(b(haystack.starts_with(&needle)))
    }
}

#[derive(Clone, Copy)]
pub struct EndsWith;
impl Function<NonFunctionValue> for EndsWith {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (haystack, needle) = require_two_bytes(args)?;
        Ok(b(haystack.ends_with(&needle)))
    }
}

#[derive(Clone, Copy)]
pub struct Equals;
impl Function<NonFunctionValue> for Equals {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (left, right) = require_two_bytes(args)?;
        Ok(b(left == right))
    }
}

#[derive(Clone, Copy)]
pub struct Count;
impl Function<NonFunctionValue> for Count {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let haystack = require_bytes(&args[0])?;
        let needle = if let Some(n) = args[1].as_number() {
            if !n.is_finite() || n < 0.0 || n > 255.0 {
                return Err(Error::PrimitiveError(format!(
                    "byte value is not in the range 0-255: {n}"
                )));
            }
            vec![n as u8]
        } else {
            require_bytes(&args[1])?
        };
        if needle.is_empty() {
            return Ok(num(0.0));
        }
        let mut count = 0;
        for i in 0..=haystack.len() - needle.len() {
            if &haystack[i..i + needle.len()] == &needle[..] {
                count += 1;
            }
        }
        Ok(num(count as f64))
    }
}

#[derive(Clone, Copy)]
pub struct Sum;
impl Function<NonFunctionValue> for Sum {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let bytes = require_one_bytes(args)?;
        let total: f64 = bytes.iter().map(|&b| f64::from(b)).sum();
        Ok(num(total))
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
        let bytes = require_one_bytes(args)?;
        match bytes.iter().copied().min() {
            Some(m) => Ok(num(f64::from(m))),
            None => Ok(NonFunctionValue::new(serde_json::Value::Null)),
        }
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
        let bytes = require_one_bytes(args)?;
        match bytes.iter().copied().max() {
            Some(m) => Ok(num(f64::from(m))),
            None => Ok(NonFunctionValue::new(serde_json::Value::Null)),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Avg;
impl Function<NonFunctionValue> for Avg {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let bytes = require_one_bytes(args)?;
        if bytes.is_empty() {
            return Ok(num(0.0));
        }
        let total: f64 = bytes.iter().map(|&b| f64::from(b)).sum();
        Ok(num(total / bytes.len() as f64))
    }
}

#[derive(Clone, Copy)]
pub struct Sort;
impl Function<NonFunctionValue> for Sort {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let mut bytes = require_one_bytes(args)?;
        bytes.sort_unstable();
        Ok(byte_array(&bytes))
    }
}

#[derive(Clone, Copy)]
pub struct FromString;
impl Function<NonFunctionValue> for FromString {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let arg = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        let input = arg.as_str().ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })?;
        Ok(byte_array(input.as_bytes()))
    }
}

#[derive(Clone, Copy)]
pub struct ToString;
impl Function<NonFunctionValue> for ToString {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let arg = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        if let Some(input) = arg.as_str() {
            return Ok(s(input));
        }
        let bytes = require_bytes(arg)?;
        String::from_utf8(bytes)
            .map(s)
            .map_err(|e| Error::PrimitiveError(format!("invalid utf-8: {e}")))
    }
}

#[derive(Clone, Copy)]
pub struct EncodeHex;
impl Function<NonFunctionValue> for EncodeHex {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let bytes = require_one_bytes(args)?;
        Ok(s(hex::encode(bytes)))
    }
}

#[derive(Clone, Copy)]
pub struct DecodeHex;
impl Function<NonFunctionValue> for DecodeHex {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let arg = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        let input = arg.as_str().ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })?;
        let decoded = hex::decode(input)
            .map_err(|e| Error::PrimitiveError(format!("invalid hex: {e}")))?;
        Ok(byte_array(&decoded))
    }
}

#[derive(Clone, Copy)]
pub struct EncodeBase64;
impl Function<NonFunctionValue> for EncodeBase64 {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let bytes = require_one_bytes(args)?;
        Ok(s(general_purpose::STANDARD.encode(&bytes)))
    }
}

#[derive(Clone, Copy)]
pub struct DecodeBase64;
impl Function<NonFunctionValue> for DecodeBase64 {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let arg = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        let input = arg.as_str().ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })?;
        let decoded = general_purpose::STANDARD
            .decode(input.as_bytes())
            .map_err(|e| Error::PrimitiveError(format!("invalid base64: {e}")))?;
        Ok(byte_array(&decoded))
    }
}
