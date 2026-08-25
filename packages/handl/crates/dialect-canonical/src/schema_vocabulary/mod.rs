mod json_schema;
mod json_schema_vocabulary;

pub use json_schema::JsonSchema;
pub use json_schema_vocabulary::{JSON_SCHEMA_VOCABULARY, JsonSchemaVocabulary};

#[cfg(test)]
mod json_schema_tests;
#[cfg(test)]
mod json_schema_vocabulary_tests;

#[cfg(test)]
mod tests;
