mod coercion;
mod module;
mod predicates;
mod utilities;

pub use module::TypeModule;

#[cfg(test)]
mod coercion_tests;
#[cfg(test)]
mod predicates_tests;
#[cfg(test)]
mod utilities_tests;
