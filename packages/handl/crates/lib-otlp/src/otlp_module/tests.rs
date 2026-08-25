use std::sync::Arc;

use handl_vm_common::Module;
use handl_vm_values_json::{CanonicalContext, NonFunctionValue};
use serde_json::Value;

use crate::otlp_module::OtlpModule;
use crate::stub::{OtlpEvent, StubOtlpState};

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v)
}

fn obj(v: Value) -> NonFunctionValue {
    NonFunctionValue::new(v)
}

fn call(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

#[test]
fn module_name() {
    let module = OtlpModule::new(Arc::new(StubOtlpState::new()));
    assert_eq!(module.name(), "otlp");
}

#[test]
fn configure_is_recorded() {
    let state = Arc::new(StubOtlpState::new());
    let module = OtlpModule::new(state.clone());
    call(
        &module,
        "configure",
        &[obj(serde_json::json!({
            "endpoint": "http://localhost:4318",
            "service_name": "test"
        }))],
    );
    assert!(matches!(state.events()[0], OtlpEvent::Configured(_)));
}

#[test]
fn start_span_returns_id() {
    let state = Arc::new(StubOtlpState::new());
    let module = OtlpModule::new(state.clone());
    let id = call(&module, "start_span", &[s("op")]);
    assert!(id.as_str().is_some());
    assert!(matches!(state.events()[0], OtlpEvent::SpanStarted(_)));
}

#[test]
fn start_span_with_parent() {
    let state = Arc::new(StubOtlpState::new());
    let module = OtlpModule::new(state.clone());
    let parent = call(
        &module,
        "start_span",
        &[
            s("parent"),
            obj(serde_json::json!({"attributes": {"a": 1}})),
        ],
    );
    let child = call(
        &module,
        "start_span",
        &[
            s("child"),
            obj(serde_json::json!({"parent": parent.as_str().unwrap()})),
        ],
    );
    assert!(child.as_str().is_some());
    assert_eq!(state.events().len(), 2);
}

#[test]
fn end_span_is_recorded() {
    let state = Arc::new(StubOtlpState::new());
    let module = OtlpModule::new(state.clone());
    let id = call(&module, "start_span", &[s("op")]);
    call(&module, "end_span", &[id]);
    assert!(matches!(state.events()[1], OtlpEvent::SpanEnded { .. }));
}

#[test]
fn set_attribute_is_recorded() {
    let state = Arc::new(StubOtlpState::new());
    let module = OtlpModule::new(state.clone());
    let id = call(&module, "start_span", &[s("op")]);
    call(
        &module,
        "set_attribute",
        &[id.clone(), s("key"), NonFunctionValue::new(42)],
    );
    assert!(matches!(state.events()[1], OtlpEvent::SpanAttribute { .. }));
}

#[test]
fn add_event_is_recorded() {
    let state = Arc::new(StubOtlpState::new());
    let module = OtlpModule::new(state.clone());
    let id = call(&module, "start_span", &[s("op")]);
    call(
        &module,
        "add_event",
        &[
            id.clone(),
            s("event"),
            obj(serde_json::json!({"attributes": {"x": true}})),
        ],
    );
    assert!(matches!(state.events()[1], OtlpEvent::SpanEvent { .. }));
}

#[test]
fn record_log_is_recorded() {
    let state = Arc::new(StubOtlpState::new());
    let module = OtlpModule::new(state.clone());
    call(
        &module,
        "record_log",
        &[s("INFO"), s("hello"), obj(serde_json::json!({"attr": "v"}))],
    );
    assert!(matches!(state.events()[0], OtlpEvent::LogRecorded { .. }));
}

#[test]
fn record_metric_is_recorded() {
    let state = Arc::new(StubOtlpState::new());
    let module = OtlpModule::new(state.clone());
    call(
        &module,
        "record_metric",
        &[
            s("requests"),
            NonFunctionValue::new(1.0),
            obj(serde_json::json!({"kind": "counter"})),
        ],
    );
    assert!(matches!(
        state.events()[0],
        OtlpEvent::MetricRecorded { .. }
    ));
}

#[test]
fn flush_and_shutdown_are_recorded() {
    let state = Arc::new(StubOtlpState::new());
    let module = OtlpModule::new(state.clone());
    call(&module, "flush", &[]);
    call(&module, "shutdown", &[]);
    assert!(matches!(state.events()[0], OtlpEvent::Flushed));
    assert!(matches!(state.events()[1], OtlpEvent::Shutdown));
}
