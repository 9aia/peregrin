mod arithmetic;
mod constants;
mod exponential;
mod helpers;
mod module;
mod predicates;
mod rounding;
mod trigonometry;

pub use module::MathModule;

#[cfg(test)]
mod arithmetic_tests;
#[cfg(test)]
mod math_tests;
