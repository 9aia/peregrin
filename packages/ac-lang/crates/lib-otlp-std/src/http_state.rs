use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use ac_lib_otlp::{
    id, MetricKind, MetricOptions, OtlpConfig, OtlpError, OtlpResult, OtlpState,
    StartSpanOptions,
};

/// Concrete OTLP/HTTP exporter that sends telemetry to an OTLP collector.
#[derive(Default)]
pub struct HttpOtlpState {
    config: Mutex<Option<OtlpConfig>>,
    spans: Mutex<HashMap<String, Span>>,
    logs: Mutex<Vec<LogRecord>>,
    metrics: Mutex<Vec<MetricRecord>>,
}

struct Span {
    id: String,
    trace_id: String,
    parent_id: Option<String>,
    name: String,
    start_time: u128,
    end_time: Option<u128>,
    attributes: HashMap<String, Value>,
    events: Vec<Event>,
}

struct Event {
    name: String,
    timestamp: u128,
    attributes: HashMap<String, Value>,
}

struct LogRecord {
    timestamp: u128,
    level: String,
    message: String,
    attributes: HashMap<String, Value>,
}

struct MetricRecord {
    timestamp: u128,
    name: String,
    value: f64,
    kind: MetricKind,
    attributes: HashMap<String, Value>,
}

impl HttpOtlpState {
    /// Create a new unconfigured exporter.
    pub fn new() -> Self {
        Self {
            config: Mutex::new(None),
            spans: Mutex::new(HashMap::new()),
            logs: Mutex::new(Vec::new()),
            metrics: Mutex::new(Vec::new()),
        }
    }
}

impl OtlpState for HttpOtlpState {
    fn configure(&self, config: OtlpConfig) -> OtlpResult<()> {
        *self.config.lock().unwrap() = Some(config);
        *self.spans.lock().unwrap() = HashMap::new();
        *self.logs.lock().unwrap() = Vec::new();
        *self.metrics.lock().unwrap() = Vec::new();
        Ok(())
    }

    fn start_span(&self, name: &str, options: StartSpanOptions) -> OtlpResult<String> {
        let mut spans = self.spans.lock().unwrap();
        let (trace_id, parent_id) = if let Some(parent) = options.parent.as_ref() {
            let parent_span = spans
                .get(parent)
                .ok_or_else(|| OtlpError::UnknownSpan { id: parent.clone() })?;
            (parent_span.trace_id.clone(), Some(parent.clone()))
        } else {
            (id::gen_trace_id(), None)
        };
        let span_id = id::gen_span_id();
        spans.insert(
            span_id.clone(),
            Span {
                id: span_id.clone(),
                trace_id,
                parent_id,
                name: name.to_string(),
                start_time: id::now_ns(),
                end_time: None,
                attributes: options.attributes,
                events: Vec::new(),
            },
        );
        Ok(span_id)
    }

    fn end_span(&self, id: &str) -> OtlpResult<()> {
        let mut spans = self.spans.lock().unwrap();
        let span = spans
            .get_mut(id)
            .ok_or_else(|| OtlpError::UnknownSpan { id: id.to_string() })?;
        span.end_time = Some(id::now_ns());
        Ok(())
    }

    fn set_attribute(&self, id: &str, key: &str, value: Value) -> OtlpResult<()> {
        let mut spans = self.spans.lock().unwrap();
        let span = spans
            .get_mut(id)
            .ok_or_else(|| OtlpError::UnknownSpan { id: id.to_string() })?;
        span.attributes.insert(key.to_string(), value);
        Ok(())
    }

    fn add_event(
        &self,
        id: &str,
        name: &str,
        attributes: HashMap<String, Value>,
    ) -> OtlpResult<()> {
        let mut spans = self.spans.lock().unwrap();
        let span = spans
            .get_mut(id)
            .ok_or_else(|| OtlpError::UnknownSpan { id: id.to_string() })?;
        span.events.push(Event {
            name: name.to_string(),
            timestamp: id::now_ns(),
            attributes,
        });
        Ok(())
    }

    fn record_log(
        &self,
        level: &str,
        message: &str,
        attributes: HashMap<String, Value>,
    ) -> OtlpResult<()> {
        self.logs.lock().unwrap().push(LogRecord {
            timestamp: id::now_ns(),
            level: level.to_string(),
            message: message.to_string(),
            attributes,
        });
        Ok(())
    }

    fn record_metric(&self, name: &str, value: f64, options: MetricOptions) -> OtlpResult<()> {
        self.metrics.lock().unwrap().push(MetricRecord {
            timestamp: id::now_ns(),
            name: name.to_string(),
            value,
            kind: options.kind,
            attributes: options.attributes,
        });
        Ok(())
    }

    fn flush(&self) -> OtlpResult<()> {
        let config = self
            .config
            .lock()
            .unwrap()
            .clone()
            .ok_or(OtlpError::NotConfigured)?;

        let ended = take_ended_spans(&self.spans);
        if !ended.is_empty() {
            send_trace_payload(&config, &ended)?;
        }

        let logs = std::mem::take(&mut *self.logs.lock().unwrap());
        if !logs.is_empty() {
            send_logs_payload(&config, &logs)?;
        }

        let metrics = std::mem::take(&mut *self.metrics.lock().unwrap());
        if !metrics.is_empty() {
            send_metrics_payload(&config, &metrics)?;
        }

        Ok(())
    }

    fn shutdown(&self) -> OtlpResult<()> {
        {
            let mut spans = self.spans.lock().unwrap();
            for span in spans.values_mut() {
                span.end_time = Some(id::now_ns());
            }
        }
        self.flush()
    }
}

fn take_ended_spans(spans: &Mutex<HashMap<String, Span>>) -> Vec<Span> {
    let mut guard = spans.lock().unwrap();
    let taken = std::mem::take(&mut *guard);
    let mut ended = Vec::new();
    let mut remaining = HashMap::new();
    for (id, span) in taken {
        if span.end_time.is_some() {
            ended.push(span);
        } else {
            remaining.insert(id, span);
        }
    }
    *guard = remaining;
    ended
}

fn attribute_value(value: &Value) -> Value {
    match value {
        Value::String(s) => json!({ "stringValue": s }),
        Value::Number(n) => json!({ "doubleValue": n.as_f64().unwrap_or(0.0) }),
        Value::Bool(b) => json!({ "boolValue": b }),
        Value::Null => json!({}),
        _ => json!({ "stringValue": value.to_string() }),
    }
}

fn attribute_array(attrs: &HashMap<String, Value>) -> Value {
    Value::Array(
        attrs
            .iter()
            .map(|(k, v)| json!({ "key": k, "value": attribute_value(v) }))
            .collect(),
    )
}

fn resource_attributes(config: &OtlpConfig) -> Value {
    let mut attrs = vec![json!({
        "key": "service.name",
        "value": { "stringValue": &config.service_name }
    })];
    for (k, v) in &config.resource_attributes {
        attrs.push(json!({ "key": k, "value": attribute_value(v) }));
    }
    Value::Array(attrs)
}

fn scope() -> Value {
    json!({
        "name": "ac-lib-otlp-std",
        "version": env!("CARGO_PKG_VERSION")
    })
}

fn post_json(config: &OtlpConfig, url: &str, payload: &Value) -> OtlpResult<()> {
    let body = serde_json::to_string(payload)
        .map_err(|e| OtlpError::serialization(e.to_string()))?;
    let mut builder = ureq::post(url);
    for (name, value) in &config.headers {
        builder = builder.set(name, value);
    }
    let resp = builder
        .set("Content-Type", "application/json")
        .send_string(&body)
        .map_err(|e| OtlpError::network(e.to_string()))?;
    if resp.status() >= 400 {
        return Err(OtlpError::network(format!("HTTP {}", resp.status())));
    }
    Ok(())
}

fn send_trace_payload(config: &OtlpConfig, spans: &[Span]) -> OtlpResult<()> {
    let now = id::now_ns();
    let payload_spans: Vec<Value> = spans
        .iter()
        .map(|s| {
            let mut obj = serde_json::Map::new();
            obj.insert("traceId".to_string(), json!(s.trace_id));
            obj.insert("spanId".to_string(), json!(s.id));
            obj.insert("name".to_string(), json!(s.name));
            obj.insert("kind".to_string(), json!(1));
            obj.insert(
                "startTimeUnixNano".to_string(),
                json!(s.start_time.to_string()),
            );
            obj.insert(
                "endTimeUnixNano".to_string(),
                json!(s.end_time.unwrap_or(now).to_string()),
            );
            obj.insert("attributes".to_string(), attribute_array(&s.attributes));
            let events: Vec<Value> = s
                .events
                .iter()
                .map(|e| {
                    json!({
                        "name": e.name,
                        "timeUnixNano": e.timestamp.to_string(),
                        "attributes": attribute_array(&e.attributes)
                    })
                })
                .collect();
            obj.insert("events".to_string(), Value::Array(events));
            if let Some(parent) = &s.parent_id {
                obj.insert("parentSpanId".to_string(), json!(parent));
            }
            Value::Object(obj)
        })
        .collect();

    let payload = json!({
        "resourceSpans": [{
            "resource": { "attributes": resource_attributes(config) },
            "scopeSpans": [{
                "scope": scope(),
                "spans": payload_spans
            }]
        }]
    });

    let url = format!("{}/v1/traces", config.endpoint.trim_end_matches('/'));
    post_json(config, &url, &payload)
}

fn send_logs_payload(config: &OtlpConfig, logs: &[LogRecord]) -> OtlpResult<()> {
    let records: Vec<Value> = logs
        .iter()
        .map(|l| {
            json!({
                "timeUnixNano": l.timestamp.to_string(),
                "severityText": l.level,
                "body": { "stringValue": l.message },
                "attributes": attribute_array(&l.attributes)
            })
        })
        .collect();

    let payload = json!({
        "resourceLogs": [{
            "resource": { "attributes": resource_attributes(config) },
            "scopeLogs": [{
                "scope": scope(),
                "logRecords": records
            }]
        }]
    });

    let url = format!("{}/v1/logs", config.endpoint.trim_end_matches('/'));
    post_json(config, &url, &payload)
}

fn send_metrics_payload(config: &OtlpConfig, metrics: &[MetricRecord]) -> OtlpResult<()> {
    let mut by_name: HashMap<String, Vec<&MetricRecord>> = HashMap::new();
    for m in metrics {
        by_name.entry(m.name.clone()).or_default().push(m);
    }

    let payload_metrics: Vec<Value> = by_name
        .iter()
        .map(|(name, records)| {
            let data_points: Vec<Value> = records
                .iter()
                .map(|m| {
                    json!({
                        "asDouble": m.value,
                        "timeUnixNano": m.timestamp.to_string(),
                        "attributes": attribute_array(&m.attributes)
                    })
                })
                .collect();
            if records.first().map(|m| m.kind).unwrap_or(MetricKind::Counter) == MetricKind::Counter
            {
                json!({
                    "name": name,
                    "sum": {
                        "dataPoints": data_points,
                        "aggregationTemporality": 2,
                        "isMonotonic": true
                    }
                })
            } else {
                json!({
                    "name": name,
                    "gauge": { "dataPoints": data_points }
                })
            }
        })
        .collect();

    let payload = json!({
        "resourceMetrics": [{
            "resource": { "attributes": resource_attributes(config) },
            "scopeMetrics": [{
                "scope": scope(),
                "metrics": payload_metrics
            }]
        }]
    });

    let url = format!("{}/v1/metrics", config.endpoint.trim_end_matches('/'));
    post_json(config, &url, &payload)
}

/// Create a preconfigured `otlp` module using [`HttpOtlpState`].
pub fn std_otlp_module() -> ac_lib_otlp::OtlpModule {
    let state: Arc<dyn OtlpState> = Arc::new(HttpOtlpState::new());
    ac_lib_otlp::OtlpModule::new(state)
}
