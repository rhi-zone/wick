//! Basic example: Parsing and evaluating scalar expressions.
//!
//! Run with: cargo run --example basic

use dew_core::Expr;
use dew_scalar::{eval, scalar_registry};
use std::collections::HashMap;

fn main() {
    // Parse a simple expression
    let expr = Expr::parse("sin(x * pi()) + cos(y)").unwrap();
    println!("Parsed expression: {:?}", expr.ast());

    // Set up variables
    let vars: HashMap<String, f32> = [("x".into(), 0.5), ("y".into(), 0.0)].into();

    // Create a registry with standard math functions
    let registry = scalar_registry();

    // Evaluate the expression
    let result = eval(expr.ast(), &vars, &registry).unwrap();
    println!("sin(0.5 * pi) + cos(0) = {}", result);
    // Should print approximately 2.0 (sin(π/2) + cos(0) = 1 + 1 = 2)

    // Try some more expressions
    let expressions = [
        ("sqrt(16)", HashMap::new()),
        ("lerp(0, 100, 0.25)", HashMap::new()),
        ("clamp(x, 0, 1)", [("x".into(), 1.5)].into()),
        ("smoothstep(0, 1, t)", [("t".into(), 0.5)].into()),
    ];

    println!("\nMore examples:");
    for (expr_str, vars) in expressions {
        let expr = Expr::parse(expr_str).unwrap();
        let result = eval(expr.ast(), &vars, &registry).unwrap();
        println!("  {} = {}", expr_str, result);
    }
}
