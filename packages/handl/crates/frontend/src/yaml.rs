use handl_source::SourceMap;
use handl_vm_values_json::{Frontend, NonFunctionValue};

pub struct YamlFrontend;
impl Frontend for YamlFrontend {
    type Error = String;
    fn parse(&self, input: &str) -> Result<NonFunctionValue, Self::Error> {
        handl_frontend_yaml::parse(input).map_err(|e| e.to_string())
    }
    fn serialize(&self, value: &NonFunctionValue) -> Result<String, Self::Error> {
        handl_frontend_yaml::to_string(value).map_err(|e| e.to_string())
    }
    fn parse_with_locations(
        &self,
        input: &str,
    ) -> Result<(NonFunctionValue, SourceMap), Self::Error> {
        handl_frontend_yaml::YamlFrontend
            .parse_with_locations(input)
            .map_err(|e| e.to_string())
    }
}
