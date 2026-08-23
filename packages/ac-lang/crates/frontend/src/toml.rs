use ac_runtime::{Frontend, NonFunctionValue};
use ac_source::SourceMap;

pub struct TomlFrontend;
impl Frontend for TomlFrontend {
    type Error = String;
    fn parse(&self, input: &str) -> Result<NonFunctionValue, Self::Error> {
        ac_frontend_toml::parse(input).map_err(|e| e.to_string())
    }
    fn serialize(&self, value: &NonFunctionValue) -> Result<String, Self::Error> {
        ac_frontend_toml::to_string(value).map_err(|e| e.to_string())
    }
    fn parse_with_locations(
        &self,
        input: &str,
    ) -> Result<(NonFunctionValue, SourceMap), Self::Error> {
        ac_frontend_toml::TomlFrontend
            .parse_with_locations(input)
            .map_err(|e| e.to_string())
    }
}
