//! Complex number example: arithmetic and functions.
//!
//! Run with: cargo run --example complex

use dew_complex::{Value, complex_registry, eval};
use dew_core::Expr;
use std::collections::HashMap;

fn main() {
    let registry = complex_registry();

    // Complex multiplication
    println!("Complex Arithmetic\n{:-<40}", "");

    let expr = Expr::parse("a * b").unwrap();
    let vars: HashMap<String, Value<f32>> = [
        ("a".into(), complex(1.0, 2.0)), // 1 + 2i
        ("b".into(), complex(3.0, 4.0)), // 3 + 4i
    ]
    .into();
    let result = eval(expr.ast(), &vars, &registry).unwrap();
    println!("(1 + 2i) * (3 + 4i) = {:?}", result);
    println!("  Expected: -5 + 10i");

    // Complex division
    let expr = Expr::parse("a / b").unwrap();
    let result = eval(expr.ast(), &vars, &registry).unwrap();
    println!("(1 + 2i) / (3 + 4i) = {:?}", result);

    // Component extraction
    println!("\nComponent Functions\n{:-<40}", "");

    let z = complex(3.0, 4.0);
    let vars: HashMap<String, Value<f32>> = [("z".into(), z)].into();

    for func in ["re(z)", "im(z)", "abs(z)", "arg(z)", "norm(z)"] {
        let expr = Expr::parse(func).unwrap();
        let result = eval(expr.ast(), &vars, &registry).unwrap();
        println!("{} for 3+4i = {:?}", func, result);
    }

    // Conjugate
    println!("\nConjugate\n{:-<40}", "");

    let expr = Expr::parse("conj(z)").unwrap();
    let result = eval(expr.ast(), &vars, &registry).unwrap();
    println!("conj(3 + 4i) = {:?}", result);

    // Euler's formula: e^(i*pi) = -1
    println!("\nEuler's Formula\n{:-<40}", "");

    let expr = Expr::parse("exp(z)").unwrap();
    let pi = std::f32::consts::PI;
    let vars: HashMap<String, Value<f32>> = [("z".into(), complex(0.0, pi))].into();
    let result = eval(expr.ast(), &vars, &registry).unwrap();
    println!("exp(i * pi) = {:?}", result);
    println!("  Expected: -1 + 0i (Euler's identity)");

    // Polar form
    println!("\nPolar Form\n{:-<40}", "");

    let expr = Expr::parse("polar(r, theta)").unwrap();
    let vars: HashMap<String, Value<f32>> = [
        ("r".into(), Value::Scalar(2.0)),
        ("theta".into(), Value::Scalar(pi / 4.0)), // 45 degrees
    ]
    .into();
    let result = eval(expr.ast(), &vars, &registry).unwrap();
    println!("polar(2, pi/4) = {:?}", result);
    println!("  Expected: sqrt(2) + sqrt(2)i ≈ 1.414 + 1.414i");

    // Complex roots
    println!("\nComplex Square Root\n{:-<40}", "");

    let expr = Expr::parse("sqrt(z)").unwrap();
    let vars: HashMap<String, Value<f32>> = [("z".into(), complex(-1.0, 0.0))].into();
    let result = eval(expr.ast(), &vars, &registry).unwrap();
    println!("sqrt(-1) = {:?}", result);
    println!("  Expected: i = 0 + 1i");
}

fn complex(re: f32, im: f32) -> Value<f32> {
    Value::Complex([re, im])
}
