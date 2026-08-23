use std::fmt;

use ac_runtime::NonFunctionValue;
use writeable::{Part, PartsWrite};

/// A `PartsWrite` implementation that collects formatted output into
/// `(text, part)` pairs, suitable for building `formatToParts`-style
/// structured results.
///
/// Text written outside any `with_part` call is tagged with
/// `Part { category: "", value: "literal" }`.
pub(super) struct PartsCollector {
    parts: Vec<(String, Part)>,
    buf: String,
    current_part: Part,
}

impl PartsCollector {
    pub fn new() -> Self {
        Self {
            parts: Vec::new(),
            buf: String::new(),
            current_part: Part {
                category: "",
                value: "literal",
            },
        }
    }

    fn flush(&mut self) {
        if !self.buf.is_empty() {
            let text = std::mem::take(&mut self.buf);
            self.parts.push((text, self.current_part));
        }
    }

    /// Returns the collected parts as `(type, value)` pairs, where
    /// `type` is the ICU4X part `value` field (e.g. `"integer"`,
    /// `"group"`, `"decimal"`, `"fraction"`, `"minusSign"`).
    pub fn into_parts(mut self) -> Vec<(String, String)> {
        self.flush();
        self.parts
            .into_iter()
            .map(|(text, part)| (part.value.to_string(), text))
            .collect()
    }

    /// Returns the collected parts as a `NonFunctionValue` array of
    /// `{ "type": ..., "value": ... }` objects.
    pub fn into_value(self) -> NonFunctionValue {
        let arr: Vec<serde_json::Value> = self
            .into_parts()
            .into_iter()
            .map(|(part_type, value)| serde_json::json!({ "type": part_type, "value": value }))
            .collect();
        NonFunctionValue::new(serde_json::Value::Array(arr))
    }
}

impl Default for PartsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Write for PartsCollector {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.buf.push_str(s);
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.buf.push(c);
        Ok(())
    }
}

impl PartsWrite for PartsCollector {
    type SubPartsWrite = PartsCollector;

    fn with_part(
        &mut self,
        part: Part,
        mut f: impl FnMut(&mut Self::SubPartsWrite) -> fmt::Result,
    ) -> fmt::Result {
        self.flush();
        let prev_part = self.current_part;
        self.current_part = part;
        let result = f(self);
        self.flush();
        self.current_part = prev_part;
        result
    }
}
