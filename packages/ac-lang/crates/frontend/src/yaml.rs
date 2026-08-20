use ac_runtime::{Frontend, RuntimeValue};
use ac_source::SourceMap;

pub struct YamlFrontend;
impl Frontend for YamlFrontend {
    type Error = String;
    fn parse(&self, input: &str) -> Result<RuntimeValue, Self::Error> {
        ac_frontend_yaml::parse(input).map_err(|e| e.to_string())
    }
    fn serialize(&self, value: &RuntimeValue) -> Result<String, Self::Error> {
        ac_frontend_yaml::to_string(value).map_err(|e| e.to_string())
    }
    fn parse_with_locations(&self, input: &str) -> Result<(RuntimeValue, SourceMap), Self::Error> {
        ac_frontend_yaml::YamlFrontend
            .parse_with_locations(input)
            .map_err(|e| e.to_string())
    }
}
