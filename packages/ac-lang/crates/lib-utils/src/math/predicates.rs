use super::helpers::{b, bool_unary_fn, require_number};

bool_unary_fn!(IsNan, |x: f64| x.is_nan());
bool_unary_fn!(IsFinite, |x: f64| x.is_finite());
bool_unary_fn!(IsInfinite, |x: f64| x.is_infinite());
bool_unary_fn!(IsInteger, |x: f64| x.is_finite() && x.fract() == 0.0);
