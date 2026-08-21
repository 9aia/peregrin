#[allow(clippy::module_inception)]
mod array;
mod higher_order;

pub use array::ArrayModule;

#[cfg(test)]
mod array_tests;
#[cfg(test)]
mod higher_order_tests;
