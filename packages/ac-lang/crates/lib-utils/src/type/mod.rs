#[allow(clippy::module_inception)]
mod r#type;

pub use r#type::TypeModule;

#[cfg(test)]
mod type_tests;
