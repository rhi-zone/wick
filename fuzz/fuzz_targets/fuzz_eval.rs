#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use dew_core::Expr;
use dew_scalar::scalar_registry;
use std::collections::HashMap;

/// Structured input for evaluation fuzzing.
#[derive(Debug, Arbitrary)]
struct EvalInput {
    /// Expression source (will be parsed, invalid ones filtered)
    expr: String,
    /// Variable values to use
    vars: Vec<(String, f32)>,
}

fuzz_target!(|input: EvalInput| {
    // Only fuzz expressions that parse successfully
    let Ok(expr) = Expr::parse(&input.expr) else {
        return;
    };

    // Build variable map from fuzzer input
    let vars: HashMap<String, f32> = input
        .vars
        .into_iter()
        .filter(|(name, _)| !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_'))
        .collect();

    // Get standard function registry
    let registry: dew_scalar::FunctionRegistry<f32> = scalar_registry();

    // Evaluation should never panic (may return Err for unknown vars/funcs, that's fine)
    let _ = dew_scalar::eval(expr.ast(), &vars, &registry);
});
