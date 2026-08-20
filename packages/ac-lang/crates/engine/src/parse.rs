use ac_frontend::{frontend_for, YamlFrontend};
use ac_runtime::Frontend;

pub fn run(input: &str, format: &str) -> Result<String, String> {
    let frontend = frontend_for(format)?;
    let value = frontend.parse(input)?;
    let output = YamlFrontend.serialize(&value)?;
    Ok(output)
}
