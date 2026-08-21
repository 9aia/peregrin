use ac_runtime::NonFunctionValue;

use super::helpers::{num, array_unary_fn};

array_unary_fn!(Sum, |input: &[serde_json::Value]| {
    let total: f64 = input.iter().filter_map(|v| v.as_f64()).sum();
    num(total)
});

array_unary_fn!(Avg, |input: &[serde_json::Value]| {
    let numbers: Vec<f64> = input.iter().filter_map(|v| v.as_f64()).collect();
    if numbers.is_empty() {
        num(0.0)
    } else {
        num(numbers.iter().sum::<f64>() / numbers.len() as f64)
    }
});

array_unary_fn!(Min, |input: &[serde_json::Value]| {
    let numbers: Vec<f64> = input.iter().filter_map(|v| v.as_f64()).collect();
    match numbers.iter().copied().min_by(|a, c| a.total_cmp(c)) {
        Some(m) => num(m),
        None => NonFunctionValue::new(serde_json::Value::Null),
    }
});

array_unary_fn!(Max, |input: &[serde_json::Value]| {
    let numbers: Vec<f64> = input.iter().filter_map(|v| v.as_f64()).collect();
    match numbers.iter().copied().max_by(|a, c| a.total_cmp(c)) {
        Some(m) => num(m),
        None => NonFunctionValue::new(serde_json::Value::Null),
    }
});

array_unary_fn!(Product, |input: &[serde_json::Value]| {
    let product: f64 = input.iter().filter_map(|v| v.as_f64()).product();
    num(product)
});

array_unary_fn!(Mean, |input: &[serde_json::Value]| {
    let numbers: Vec<f64> = input.iter().filter_map(|v| v.as_f64()).collect();
    if numbers.is_empty() {
        num(0.0)
    } else {
        num(numbers.iter().sum::<f64>() / numbers.len() as f64)
    }
});

array_unary_fn!(Median, |input: &[serde_json::Value]| {
    let mut numbers: Vec<f64> = input.iter().filter_map(|v| v.as_f64()).collect();
    if numbers.is_empty() {
        return num(0.0);
    }
    numbers.sort_by(|a, b| a.total_cmp(b));
    let len = numbers.len();
    if len % 2 == 0 {
        num((numbers[len / 2 - 1] + numbers[len / 2]) / 2.0)
    } else {
        num(numbers[len / 2])
    }
});

array_unary_fn!(Stddev, |input: &[serde_json::Value]| {
    let numbers: Vec<f64> = input.iter().filter_map(|v| v.as_f64()).collect();
    if numbers.len() < 2 {
        return num(0.0);
    }
    let mean = numbers.iter().sum::<f64>() / numbers.len() as f64;
    let variance = numbers.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / numbers.len() as f64;
    num(variance.sqrt())
});
