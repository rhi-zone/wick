//! Conditionals example: Using if/then/else and comparisons.
//!
//! Run with: cargo run --example conditionals

use dew_core::Expr;
use dew_scalar::{eval, scalar_registry};
use std::collections::HashMap;

fn main() {
    let registry = scalar_registry();

    // Simple conditional
    let expr = Expr::parse("if x > 0 then 1 else -1").unwrap();
    println!("Expression: if x > 0 then 1 else -1");
    println!(
        "  x = 5:  {}",
        eval(expr.ast(), &[("x".into(), 5.0)].into(), &registry).unwrap()
    );
    println!(
        "  x = -3: {}",
        eval(expr.ast(), &[("x".into(), -3.0)].into(), &registry).unwrap()
    );

    // Absolute value using conditional
    let abs_expr = Expr::parse("if x >= 0 then x else -x").unwrap();
    println!("\nAbsolute value: if x >= 0 then x else -x");
    for x in [-5.0, 0.0, 3.0] {
        let vars: HashMap<String, f32> = [("x".into(), x)].into();
        let result = eval(abs_expr.ast(), &vars, &registry).unwrap();
        println!("  |{}| = {}", x, result);
    }

    // Compound conditions with boolean logic
    let expr = Expr::parse("if x > 0 and x < 10 then x else 0").unwrap();
    println!("\nClamping to (0, 10): if x > 0 and x < 10 then x else 0");
    for x in [-5.0, 5.0, 15.0] {
        let vars: HashMap<String, f32> = [("x".into(), x)].into();
        let result = eval(expr.ast(), &vars, &registry).unwrap();
        println!("  x = {} -> {}", x, result);
    }

    // Nested conditionals
    let expr = Expr::parse("if x > 10 then 2 else if x > 0 then 1 else 0").unwrap();
    println!("\nClassify: if x > 10 then 2 else if x > 0 then 1 else 0");
    for x in [-5.0, 5.0, 15.0] {
        let vars: HashMap<String, f32> = [("x".into(), x)].into();
        let result = eval(expr.ast(), &vars, &registry).unwrap();
        println!("  classify({}) = {}", x, result);
    }

    // Using `not` and `or`
    let expr = Expr::parse("if not (x == 0) then 1 / x else 0").unwrap();
    println!("\nSafe division: if not (x == 0) then 1 / x else 0");
    for x in [0.0, 2.0, 5.0] {
        let vars: HashMap<String, f32> = [("x".into(), x)].into();
        let result = eval(expr.ast(), &vars, &registry).unwrap();
        println!("  1/{} = {}", x, result);
    }
}
