use std::collections::HashMap;
use std::sync::Arc;

use ac_runtime::{Context, Function};
use ac_runtime_modules::Module;
use ac_runtime_values::{Error as VmError, NonFunctionValue};
use serde_json::Value;

use crate::state::{MetricKind, MetricOptions, OtlpConfig, OtlpState, StartSpanOptions};

/// Module adapter for [`OtlpState`] — exposes `otlp.*` functions.
///
/// | Function | Args | Description |
/// |----------|------|-------------|
/// | `otlp.configure(config)` | `(config)` | Set endpoint, service name, headers and resource attributes. |
/// | `otlp.start_span(name, options?)` | `(name, options?)` | Start a span and return its id. |
/// | `otlp.end_span(id)` | `(id)` | End a span. |
/// | `otlp.set_attribute(id, key, value)` | `(id, key, value)` | Set a span attribute. |
/// | `otlp.add_event(id, name, attrs?)` | `(id, name, attrs?)` | Add a named event to a span. |
/// | `otlp.record_log(level, message, attrs?)` | `(level, message, attrs?)` | Record a log entry. |
/// | `otlp.record_metric(name, value, options?)` | `(name, value, options?)` | Record a numeric metric. |
/// | `otlp.flush()` | `()` | Export all buffered telemetry. |
/// | `otlp.shutdown()` | `()` | End all spans, flush and release the exporter. |
pub struct OtlpModule {
    configure_fn: ConfigureFn,
    start_span_fn: StartSpanFn,
    end_span_fn: EndSpanFn,
    set_attr_fn: SetAttributeFn,
    add_event_fn: AddEventFn,
    record_log_fn: RecordLogFn,
    record_metric_fn: RecordMetricFn,
    flush_fn: FlushFn,
    shutdown_fn: ShutdownFn,
}

impl OtlpModule {
    /// Create a new module backed by the given state.
    pub fn new(state: Arc<dyn OtlpState>) -> Self {
        Self {
            configure_fn: ConfigureFn { state: state.clone() },
            start_span_fn: StartSpanFn { state: state.clone() },
            end_span_fn: EndSpanFn { state: state.clone() },
            set_attr_fn: SetAttributeFn { state: state.clone() },
            add_event_fn: AddEventFn { state: state.clone() },
            record_log_fn: RecordLogFn { state: state.clone() },
            record_metric_fn: RecordMetricFn { state: state.clone() },
            flush_fn: FlushFn { state: state.clone() },
            shutdown_fn: ShutdownFn { state },
        }
    }
}

impl Module for OtlpModule {
    fn name(&self) -> &str {
        "otlp"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "configure" => Some(&self.configure_fn),
            "start_span" => Some(&self.start_span_fn),
            "end_span" => Some(&self.end_span_fn),
            "set_attribute" => Some(&self.set_attr_fn),
            "add_event" => Some(&self.add_event_fn),
            "record_log" => Some(&self.record_log_fn),
            "record_metric" => Some(&self.record_metric_fn),
            "flush" => Some(&self.flush_fn),
            "shutdown" => Some(&self.shutdown_fn),
            _ => None,
        }
    }
}

fn require_string_at(args: &[NonFunctionValue], idx: usize) -> Result<&str, VmError> {
    args.get(idx)
        .ok_or(VmError::ArgumentCount {
            expected: idx + 1,
            got: args.len(),
        })?
        .as_str()
        .ok_or(VmError::TypeMismatch {
            expected: "string",
            got: "non-string",
        })
}

fn require_number_at(args: &[NonFunctionValue], idx: usize) -> Result<f64, VmError> {
    args.get(idx)
        .ok_or(VmError::ArgumentCount {
            expected: idx + 1,
            got: args.len(),
        })?
        .as_number()
        .ok_or(VmError::TypeMismatch {
            expected: "number",
            got: "non-number",
        })
}

fn require_object_at(
    args: &[NonFunctionValue],
    idx: usize,
) -> Result<&serde_json::Map<String, Value>, VmError> {
    args.get(idx)
        .ok_or(VmError::ArgumentCount {
            expected: idx + 1,
            got: args.len(),
        })?
        .as_object()
        .ok_or(VmError::TypeMismatch {
            expected: "object",
            got: "non-object",
        })
}

fn optional_object_at(
    args: &[NonFunctionValue],
    idx: usize,
) -> Option<&serde_json::Map<String, Value>> {
    args.get(idx).and_then(|v| v.as_object())
}

fn get_string<'a>(obj: &'a serde_json::Map<String, Value>, key: &str) -> Option<&'a str> {
    obj.get(key).and_then(|v| v.as_str())
}

fn value_map_to_hashmap(map: &serde_json::Map<String, Value>) -> HashMap<String, Value> {
    map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}

fn parse_headers(map: &serde_json::Map<String, Value>) -> Result<HashMap<String, String>, VmError> {
    map.iter()
        .map(|(k, v)| {
            v.as_str()
                .map(|s| (k.clone(), s.to_string()))
                .ok_or(VmError::TypeMismatch {
                    expected: "string",
                    got: "non-string",
                })
        })
        .collect()
}

fn parse_config(args: &[NonFunctionValue]) -> Result<OtlpConfig, VmError> {
    let obj = require_object_at(args, 0)?;
    let endpoint = get_string(obj, "endpoint").unwrap_or("").to_string();
    let service_name = get_string(obj, "service_name").unwrap_or("ac-lang").to_string();
    let resource_attributes = obj
        .get("resource_attributes")
        .and_then(|v| v.as_object())
        .map(value_map_to_hashmap)
        .unwrap_or_default();
    let headers = obj
        .get("headers")
        .and_then(|v| v.as_object())
        .map(parse_headers)
        .transpose()?
        .unwrap_or_default();
    Ok(OtlpConfig {
        endpoint,
        service_name,
        resource_attributes,
        headers,
    })
}

fn parse_attributes(obj: &serde_json::Map<String, Value>, key: &str) -> HashMap<String, Value> {
    obj.get(key)
        .and_then(|v| v.as_object())
        .map(value_map_to_hashmap)
        .unwrap_or_default()
}

struct ConfigureFn {
    state: Arc<dyn OtlpState>,
}

impl Function<NonFunctionValue> for ConfigureFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, VmError> {
        let config = parse_config(args)?;
        self.state.configure(config).map_err(VmError::from)?;
        Ok(NonFunctionValue::new(Value::Null))
    }
}

struct StartSpanFn {
    state: Arc<dyn OtlpState>,
}

impl Function<NonFunctionValue> for StartSpanFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, VmError> {
        let name = require_string_at(args, 0)?;
        let mut options = StartSpanOptions::default();
        if let Some(obj) = optional_object_at(args, 1) {
            options.parent = get_string(obj, "parent").map(|s| s.to_string());
            options.attributes = parse_attributes(obj, "attributes");
        }
        let id = self.state.start_span(name, options).map_err(VmError::from)?;
        Ok(NonFunctionValue::new(id))
    }
}

struct EndSpanFn {
    state: Arc<dyn OtlpState>,
}

impl Function<NonFunctionValue> for EndSpanFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, VmError> {
        let id = require_string_at(args, 0)?;
        self.state.end_span(id).map_err(VmError::from)?;
        Ok(NonFunctionValue::new(Value::Null))
    }
}

struct SetAttributeFn {
    state: Arc<dyn OtlpState>,
}

impl Function<NonFunctionValue> for SetAttributeFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, VmError> {
        let id = require_string_at(args, 0)?;
        let key = require_string_at(args, 1)?;
        let value = args
            .get(2)
            .ok_or(VmError::ArgumentCount {
                expected: 3,
                got: args.len(),
            })?
            .inner()
            .clone();
        self.state
            .set_attribute(id, key, value)
            .map_err(VmError::from)?;
        Ok(NonFunctionValue::new(Value::Null))
    }
}

struct AddEventFn {
    state: Arc<dyn OtlpState>,
}

impl Function<NonFunctionValue> for AddEventFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, VmError> {
        let id = require_string_at(args, 0)?;
        let name = require_string_at(args, 1)?;
        let attributes = args
            .get(2)
            .and_then(|v| v.as_object())
            .map(value_map_to_hashmap)
            .unwrap_or_default();
        self.state
            .add_event(id, name, attributes)
            .map_err(VmError::from)?;
        Ok(NonFunctionValue::new(Value::Null))
    }
}

struct RecordLogFn {
    state: Arc<dyn OtlpState>,
}

impl Function<NonFunctionValue> for RecordLogFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, VmError> {
        let level = require_string_at(args, 0)?;
        let message = require_string_at(args, 1)?;
        let attributes = args
            .get(2)
            .and_then(|v| v.as_object())
            .map(value_map_to_hashmap)
            .unwrap_or_default();
        self.state
            .record_log(level, message, attributes)
            .map_err(VmError::from)?;
        Ok(NonFunctionValue::new(Value::Null))
    }
}

struct RecordMetricFn {
    state: Arc<dyn OtlpState>,
}

impl Function<NonFunctionValue> for RecordMetricFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, VmError> {
        let name = require_string_at(args, 0)?;
        let value = require_number_at(args, 1)?;
        let mut options = MetricOptions::default();
        if let Some(obj) = optional_object_at(args, 2) {
            options.kind = get_string(obj, "kind")
                .and_then(MetricKind::parse)
                .unwrap_or(MetricKind::Counter);
            options.attributes = parse_attributes(obj, "attributes");
        }
        self.state
            .record_metric(name, value, options)
            .map_err(VmError::from)?;
        Ok(NonFunctionValue::new(Value::Null))
    }
}

struct FlushFn {
    state: Arc<dyn OtlpState>,
}

impl Function<NonFunctionValue> for FlushFn {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, VmError> {
        self.state.flush().map_err(VmError::from)?;
        Ok(NonFunctionValue::new(Value::Null))
    }
}

struct ShutdownFn {
    state: Arc<dyn OtlpState>,
}

impl Function<NonFunctionValue> for ShutdownFn {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, VmError> {
        self.state.shutdown().map_err(VmError::from)?;
        Ok(NonFunctionValue::new(Value::Null))
    }
}

#[cfg(test)]
mod tests;
