use handl_vm::{Context, Function};
use handl_vm_values_json::{Error, NonFunctionValue};

use super::helpers::num;

#[derive(Clone, Copy)]
pub struct Pi;
impl Function<NonFunctionValue> for Pi {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        Ok(num(std::f64::consts::PI))
    }
}

#[derive(Clone, Copy)]
pub struct E;
impl Function<NonFunctionValue> for E {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        Ok(num(std::f64::consts::E))
    }
}

#[derive(Clone, Copy)]
pub struct Nan;
impl Function<NonFunctionValue> for Nan {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        Ok(num(f64::NAN))
    }
}

#[derive(Clone, Copy)]
pub struct Infinity;
impl Function<NonFunctionValue> for Infinity {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        Ok(num(f64::INFINITY))
    }
}

#[derive(Clone, Copy)]
pub struct Tau;
impl Function<NonFunctionValue> for Tau {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        Ok(num(std::f64::consts::TAU))
    }
}
