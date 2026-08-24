use std::time::{SystemTime, UNIX_EPOCH};

/// Generates a 16-byte (32 hex chars) OTLP trace id.
pub fn gen_trace_id() -> String {
    hex::encode(rand::random::<[u8; 16]>())
}

/// Generates an 8-byte (16 hex chars) OTLP span id.
pub fn gen_span_id() -> String {
    hex::encode(rand::random::<[u8; 8]>())
}

/// Current time as Unix nanoseconds.
pub fn now_ns() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
