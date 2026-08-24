use ac_lib_json::Json;
use ac_lib_serialize::{deserialize, serialize};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value = json!({
        "hello": "world",
        "count": 42,
        "list": [1, 2, 3]
    });

    let text = serialize(&value, &Json)?;
    println!("{}", text);

    let back = deserialize(text.as_str(), &Json)?;
    assert_eq!(value, back);

    Ok(())
}
