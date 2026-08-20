#![no_main]

use ac_fields::{interpolate_scoped, Namespaces, NoFunctions};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: String| {
    let mut ns = Namespaces::new();
    ns.insert(
        "doc".to_string(),
        serde_json::json!({"permission_mode": "read-only"}),
    );

    let inheritable = vec!["permission_mode".to_string()];

    let value = serde_json::json!({
        "message": data,
        "nested": {
            "message": data,
            "deep": {
                "message": data
            }
        }
    });

    let _ = interpolate_scoped(&value, &ns, &inheritable, &NoFunctions, &ac_dialect_canonical::AC_DIALECT);
});
