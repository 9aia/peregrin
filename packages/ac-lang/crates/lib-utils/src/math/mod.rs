#[allow(clippy::module_inception)]
mod math;

pub use math::MathModule;

#[cfg(test)]
mod math_tests;
