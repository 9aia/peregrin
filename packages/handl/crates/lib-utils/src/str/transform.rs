use handl_vm_values_json::NonFunctionValue;

use handl_vm::{Context, Function};
use handl_vm_values_json::Error;

use super::common::{require_one_string, require_string, require_two_strings, s};

#[derive(Clone, Copy)]
pub struct Split;
impl Function<NonFunctionValue> for Split {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (input, sep) = require_two_strings(args)?;
        let parts: Vec<serde_json::Value> = input
            .split(sep)
            .map(|part| serde_json::Value::String(part.to_string()))
            .collect();
        Ok(NonFunctionValue::new(serde_json::Value::Array(parts)))
    }
}

#[derive(Clone, Copy)]
pub struct Join;
impl Function<NonFunctionValue> for Join {
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
        let sep = require_string(&args[1])?;
        let arr = args[0].as_array().ok_or(Error::TypeMismatch {
            expected: "array",
            got: "non-array",
        })?;
        let parts: Vec<&str> = arr
            .iter()
            .map(|v| {
                v.as_str().ok_or(Error::TypeMismatch {
                    expected: "string",
                    got: "non-string",
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(s(parts.join(sep)))
    }
}

#[derive(Clone, Copy)]
pub struct Replace;
impl Function<NonFunctionValue> for Replace {
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
        let input = require_string(&args[0])?;
        let from = require_string(&args[1])?;
        let to = require_string(&args[2])?;
        Ok(s(input.replace(from, to)))
    }
}

#[derive(Clone, Copy)]
pub struct ReplaceFirst;
impl Function<NonFunctionValue> for ReplaceFirst {
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
        let input = require_string(&args[0])?;
        let from = require_string(&args[1])?;
        let to = require_string(&args[2])?;
        Ok(s(input.replacen(from, to, 1)))
    }
}

#[derive(Clone, Copy)]
pub struct Repeat;
impl Function<NonFunctionValue> for Repeat {
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
        let input = require_string(&args[0])?;
        let count = args[1].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })?;
        if count < 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-negative number",
                got: "negative number",
            });
        }
        if !count.is_finite() || count > 1_000_000.0 {
            return Err(Error::TypeMismatch {
                expected: "finite number <= 1_000_000",
                got: "non-finite or excessively large number",
            });
        }
        Ok(s(input.repeat(count as usize)))
    }
}

#[derive(Clone, Copy)]
pub struct CharAt;
impl Function<NonFunctionValue> for CharAt {
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
        let input = require_string(&args[0])?;
        let index = args[1].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })?;
        if index < 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-negative number",
                got: "negative number",
            });
        }
        let char_count = input.chars().count();
        if (index as usize) >= char_count {
            return Err(Error::PrimitiveError(format!(
                "index {index} out of bounds (char count: {char_count})"
            )));
        }
        Ok(s(input.chars().nth(index as usize).unwrap().to_string()))
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
        let input = require_string(&args[0])?;
        let start = args[1].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })?;
        let end = args[2].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })?;
        if start < 0.0 || end < 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-negative number",
                got: "negative number",
            });
        }
        let chars: Vec<char> = input.chars().collect();
        let end = (end as usize).min(chars.len());
        let start = (start as usize).min(end);
        Ok(s(chars[start..end].iter().collect::<String>()))
    }
}

fn pad_to_len(input: &str, target_len: usize, pad_char: &str, pad_left: bool) -> String {
    let char_count = input.chars().count();
    if char_count >= target_len || pad_char.is_empty() {
        return input.to_string();
    }
    let pad_total = target_len - char_count;
    let pad_chars: Vec<char> = pad_char.chars().collect();
    let pad_repeated: String = pad_chars.iter().cycle().take(pad_total).collect();
    if pad_left {
        format!("{pad_repeated}{input}")
    } else {
        format!("{input}{pad_repeated}")
    }
}

#[derive(Clone, Copy)]
pub struct PadLeft;
impl Function<NonFunctionValue> for PadLeft {
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
        let input = require_string(&args[0])?;
        let target_len = args[1].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })?;
        if target_len < 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-negative number",
                got: "negative number",
            });
        }
        if !target_len.is_finite() || target_len > 1_000_000.0 {
            return Err(Error::TypeMismatch {
                expected: "finite number <= 1_000_000",
                got: "non-finite or excessively large number",
            });
        }
        let pad_char = require_string(&args[2])?;
        Ok(s(pad_to_len(input, target_len as usize, pad_char, true)))
    }
}

#[derive(Clone, Copy)]
pub struct PadRight;
impl Function<NonFunctionValue> for PadRight {
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
        let input = require_string(&args[0])?;
        let target_len = args[1].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })?;
        if target_len < 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-negative number",
                got: "negative number",
            });
        }
        if !target_len.is_finite() || target_len > 1_000_000.0 {
            return Err(Error::TypeMismatch {
                expected: "finite number <= 1_000_000",
                got: "non-finite or excessively large number",
            });
        }
        let pad_char = require_string(&args[2])?;
        Ok(s(pad_to_len(input, target_len as usize, pad_char, false)))
    }
}

#[derive(Clone, Copy)]
pub struct StripPrefix;
impl Function<NonFunctionValue> for StripPrefix {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (input, prefix) = require_two_strings(args)?;
        Ok(s(input.strip_prefix(prefix).unwrap_or(input).to_string()))
    }
}

#[derive(Clone, Copy)]
pub struct StripSuffix;
impl Function<NonFunctionValue> for StripSuffix {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (input, suffix) = require_two_strings(args)?;
        Ok(s(input.strip_suffix(suffix).unwrap_or(input).to_string()))
    }
}

#[derive(Clone, Copy)]
pub struct Lines;
impl Function<NonFunctionValue> for Lines {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let lines: Vec<serde_json::Value> = if input.is_empty() {
            vec![serde_json::Value::String(String::new())]
        } else {
            input
                .lines()
                .map(|line| serde_json::Value::String(line.to_string()))
                .collect()
        };
        Ok(NonFunctionValue::new(serde_json::Value::Array(lines)))
    }
}

#[derive(Clone, Copy)]
pub struct SplitAt;
impl Function<NonFunctionValue> for SplitAt {
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
        let input = require_string(&args[0])?;
        let index = args[1].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })?;
        if index < 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-negative number",
                got: "negative number",
            });
        }
        let chars: Vec<char> = input.chars().collect();
        let mid = (index as usize).min(chars.len());
        let left: String = chars[..mid].iter().collect();
        let right: String = chars[mid..].iter().collect();
        let parts = vec![
            serde_json::Value::String(left),
            serde_json::Value::String(right),
        ];
        Ok(NonFunctionValue::new(serde_json::Value::Array(parts)))
    }
}

#[derive(Clone, Copy)]
pub struct SplitWhitespace;
impl Function<NonFunctionValue> for SplitWhitespace {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let parts: Vec<serde_json::Value> = input
            .split_whitespace()
            .map(|part| serde_json::Value::String(part.to_string()))
            .collect();
        Ok(NonFunctionValue::new(serde_json::Value::Array(parts)))
    }
}

#[derive(Clone, Copy)]
pub struct Truncate;
impl Function<NonFunctionValue> for Truncate {
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
        let input = require_string(&args[0])?;
        let max_len = args[1].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })?;
        if max_len < 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-negative number",
                got: "negative number",
            });
        }
        let max_len = max_len as usize;
        let suffix = if args.len() >= 3 {
            require_string(&args[2])?
        } else {
            "..."
        };
        let char_count = input.chars().count();
        if char_count <= max_len {
            return Ok(s(input.to_string()));
        }
        let suffix_len = suffix.chars().count();
        if max_len < suffix_len {
            let truncated: String = input.chars().take(max_len).collect();
            return Ok(s(truncated));
        }
        let take = max_len - suffix_len;
        let truncated: String = input.chars().take(take).collect();
        Ok(s(format!("{truncated}{suffix}")))
    }
}

#[derive(Clone, Copy)]
pub struct Slugify;
impl Function<NonFunctionValue> for Slugify {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let slug: String = input
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() {
                    c
                } else if c.is_whitespace() || c == '-' || c == '_' {
                    '-'
                } else {
                    '\0'
                }
            })
            .filter(|c| *c != '\0')
            .collect::<String>()
            .trim_matches('-')
            .to_string();
        let mut result = String::new();
        let mut prev_dash = false;
        for ch in slug.chars() {
            if ch == '-' {
                if !prev_dash {
                    result.push(ch);
                    prev_dash = true;
                }
            } else {
                result.push(ch);
                prev_dash = false;
            }
        }
        Ok(s(result))
    }
}

#[derive(Clone, Copy)]
pub struct EscapeHtml;
impl Function<NonFunctionValue> for EscapeHtml {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let escaped: String = input
            .chars()
            .map(|c| match c {
                '&' => "&amp;".to_string(),
                '<' => "&lt;".to_string(),
                '>' => "&gt;".to_string(),
                '"' => "&quot;".to_string(),
                '\'' => "&#x27;".to_string(),
                '/' => "&#x2F;".to_string(),
                _ => c.to_string(),
            })
            .collect();
        Ok(s(escaped))
    }
}

#[derive(Clone, Copy)]
pub struct UnescapeHtml;
impl Function<NonFunctionValue> for UnescapeHtml {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let unescaped = input
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#x27;", "'")
            .replace("&#x2F;", "/")
            .replace("&#39;", "'")
            .replace("&#47;", "/");
        Ok(s(unescaped))
    }
}

fn is_url_unreserved(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'-' || c == b'_' || c == b'.' || c == b'~'
}

#[derive(Clone, Copy)]
pub struct EscapeUrl;
impl Function<NonFunctionValue> for EscapeUrl {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let escaped: String = input
            .bytes()
            .map(|b| {
                if is_url_unreserved(b) {
                    (b as char).to_string()
                } else {
                    format!("%{b:02X}")
                }
            })
            .collect();
        Ok(s(escaped))
    }
}

#[derive(Clone, Copy)]
pub struct UnescapeUrl;
impl Function<NonFunctionValue> for UnescapeUrl {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let bytes = input.as_bytes();
        let mut result = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                let hex = &input[i + 1..i + 3];
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    result.push(byte);
                    i += 3;
                    continue;
                }
            }
            result.push(bytes[i]);
            i += 1;
        }
        let decoded = String::from_utf8(result).map_err(|_| Error::TypeMismatch {
            expected: "valid UTF-8 after URL decoding",
            got: "invalid UTF-8 sequence",
        })?;
        Ok(s(decoded))
    }
}

#[derive(Clone, Copy)]
pub struct WordWrap;
impl Function<NonFunctionValue> for WordWrap {
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
        let input = require_string(&args[0])?;
        let width = args[1].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })?;
        if width < 1.0 {
            return Err(Error::TypeMismatch {
                expected: "positive number",
                got: "non-positive number",
            });
        }
        let width = width as usize;
        let mut result = String::new();
        for line in input.lines() {
            if line.is_empty() {
                result.push('\n');
                continue;
            }
            let mut current_len = 0usize;
            for word in line.split_whitespace() {
                let word_len = word.chars().count();
                if current_len == 0 {
                    result.push_str(word);
                    current_len = word_len;
                } else if current_len + 1 + word_len <= width {
                    result.push(' ');
                    result.push_str(word);
                    current_len += 1 + word_len;
                } else {
                    result.push('\n');
                    result.push_str(word);
                    current_len = word_len;
                }
            }
            result.push('\n');
        }
        if result.ends_with('\n') {
            result.pop();
        }
        Ok(s(result))
    }
}
