use ac_runtime::{Frontend, RuntimeValue};
use ac_source::SourceMap;

pub struct JsonFrontend;
impl Frontend for JsonFrontend {
    type Error = String;
    fn parse(&self, input: &str) -> Result<RuntimeValue, Self::Error> {
        ac_frontend_json::parse(input).map_err(|e| e.to_string())
    }
    fn serialize(&self, value: &RuntimeValue) -> Result<String, Self::Error> {
        ac_frontend_json::to_string_pretty(value).map_err(|e| e.to_string())
    }
    fn parse_with_locations(&self, input: &str) -> Result<(RuntimeValue, SourceMap), Self::Error> {
        ac_frontend_json::JsonFrontend
            .parse_with_locations(input)
            .map_err(|e| e.to_string())
    }
}
