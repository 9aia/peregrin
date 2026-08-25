//! # termination
//!
//! Compile-time call-graph analysis to detect non-terminating functions.
//!
//! This module builds a static call graph from document-declared function
//! bodies and detects cycles that would cause infinite recursion at
//! runtime. Cycles through host builtins (like `add`, `mul`) are safe
//! because host functions don't call back into user-defined functions.
//!
//! ## Architectural rule
//!
//! > **A function that unconditionally calls itself (directly or via a
//! cycle with no host-function leaf) will never terminate.**

use std::collections::{HashMap, HashSet};

use handl_dialect::Dialect;
use handl_typecheck_core::TypeError;
use handl_typecheck_infer::{ExprAst, parse_expr};
use serde_json::Value as JsonValue;

/// Extract all function call names from an [`ExprAst`] recursively.
fn collect_calls(ast: &ExprAst, out: &mut HashSet<String>) {
    match ast {
        ExprAst::Literal(_) => {}
        ExprAst::Path(_) => {}
        ExprAst::Call(name, args) => {
            out.insert(name.clone());
            for arg in args {
                collect_calls(arg, out);
            }
        }
        ExprAst::Handle { body, .. } => {
            collect_calls(body, out);
        }
    }
}

/// A call graph mapping function names to the set of functions they call.
type CallGraph = HashMap<String, HashSet<String>>;

/// Build a call graph from document-declared function bodies.
///
/// `function_bodies` is a map of function name → body expression string.
/// Functions whose body cannot be parsed are skipped (they'll be caught
/// by the expression typechecker).
fn build_call_graph(function_bodies: &HashMap<String, String>, dialect: &dyn Dialect) -> CallGraph {
    let mut graph: CallGraph = HashMap::new();

    for (name, body) in function_bodies {
        let mut callees = HashSet::new();
        if let Some(ast) = parse_expr(body, dialect) {
            collect_calls(&ast, &mut callees);
        }
        graph.insert(name.clone(), callees);
    }

    graph
}

/// Detect cycles in a call graph using DFS with three-color marking.
///
/// Returns a list of unique cycles, each represented as a vector of
/// function names forming the cycle path (e.g. `["foo", "bar", "foo"]`).
fn detect_cycles(graph: &CallGraph) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut white: HashSet<String> = graph.keys().cloned().collect();
    let mut gray: HashSet<String> = HashSet::new();
    let mut black: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = Vec::new();

    while let Some(start) = white.iter().next().cloned() {
        dfs_cycle(
            &start,
            graph,
            &mut white,
            &mut gray,
            &mut black,
            &mut stack,
            &mut cycles,
        );
    }

    deduplicate_cycles(cycles)
}

/// Normalize a cycle by rotating to start from its lexicographically
/// smallest element, then deduplicate.
fn deduplicate_cycles(cycles: Vec<Vec<String>>) -> Vec<Vec<String>> {
    let mut seen: HashSet<Vec<String>> = HashSet::new();
    let mut result = Vec::new();

    for mut cycle in cycles {
        // The last element is a repeat of the first (e.g. ["a", "b", "a"]).
        // Remove it for normalization, then re-add after rotating.
        if cycle.len() > 1 {
            cycle.pop();
        }

        // Rotate to start from the lexicographically smallest element.
        let min_pos = cycle
            .iter()
            .enumerate()
            .min_by_key(|(_, v)| *v)
            .map(|(i, _)| i)
            .unwrap_or(0);
        cycle.rotate_left(min_pos);

        // Re-add the closing element.
        let first = cycle[0].clone();
        cycle.push(first);

        if seen.insert(cycle.clone()) {
            result.push(cycle);
        }
    }

    result
}

fn dfs_cycle(
    node: &str,
    graph: &CallGraph,
    white: &mut HashSet<String>,
    gray: &mut HashSet<String>,
    black: &mut HashSet<String>,
    stack: &mut Vec<String>,
    cycles: &mut Vec<Vec<String>>,
) {
    white.remove(node);
    gray.insert(node.to_string());
    stack.push(node.to_string());

    if let Some(callees) = graph.get(node) {
        for callee in callees {
            if black.contains(callee) {
                continue;
            }
            if gray.contains(callee) {
                let pos = stack.iter().position(|n| n == callee).unwrap_or(0);
                let cycle: Vec<String> = stack[pos..]
                    .iter()
                    .cloned()
                    .chain(std::iter::once(callee.clone()))
                    .collect();
                cycles.push(cycle);
                continue;
            }
            if white.contains(callee) {
                dfs_cycle(callee, graph, white, gray, black, stack, cycles);
            }
        }
    }

    gray.remove(node);
    black.insert(node.to_string());
    stack.pop();
}

/// Extract function bodies from the document data.
///
/// Functions are stored as JSON objects with a `body` string field,
/// typically under `doc.functions.<name>` or directly as field values
/// with `type: "function"`.
pub fn extract_function_bodies(data: &JsonValue) -> HashMap<String, String> {
    let mut bodies = HashMap::new();

    if let Some(obj) = data.as_object() {
        for (name, val) in obj {
            if let Some(body) = val.get("body").and_then(|v| v.as_str()) {
                bodies.insert(name.clone(), body.to_string());
            }
        }

        if let Some(functions) = obj.get("functions").and_then(|v| v.as_object()) {
            for (name, val) in functions {
                if let Some(body) = val.get("body").and_then(|v| v.as_str()) {
                    bodies.insert(name.clone(), body.to_string());
                }
            }
        }
    }

    bodies
}

/// Check all document-declared functions for infinite recursion cycles.
///
/// Returns a list of [`TypeError`]s, one per detected cycle.
pub fn check(function_bodies: &HashMap<String, String>, dialect: &dyn Dialect) -> Vec<TypeError> {
    let graph = build_call_graph(function_bodies, dialect);
    let cycles = detect_cycles(&graph);

    cycles
        .into_iter()
        .map(|cycle| {
            let cycle_str = cycle.join(" -> ");
            TypeError::custom(
                &cycle[0],
                format!("infinite recursion detected at compile time: {cycle_str}"),
                "termination analysis",
            )
        })
        .collect()
}

#[cfg(test)]
mod lib_tests;
