//! Linear algebra example: vectors, matrices, and operations.
//!
//! Run with: cargo run --example linalg

use dew_core::Expr;
use dew_linalg::{Value, eval, linalg_registry};
use std::collections::HashMap;

fn main() {
    let registry = linalg_registry();

    // Vector operations
    println!("Vector Operations\n{:-<40}", "");

    let examples = [
        (
            "dot(a, b)",
            vec![("a", vec2(1.0, 0.0)), ("b", vec2(0.0, 1.0))],
        ),
        (
            "dot(a, b)",
            vec![("a", vec2(1.0, 0.0)), ("b", vec2(1.0, 0.0))],
        ),
        ("length(v)", vec![("v", vec2(3.0, 4.0))]),
        ("normalize(v)", vec![("v", vec2(3.0, 4.0))]),
        (
            "distance(a, b)",
            vec![("a", vec2(0.0, 0.0)), ("b", vec2(3.0, 4.0))],
        ),
    ];

    for (expr_str, var_list) in examples {
        let expr = Expr::parse(expr_str).unwrap();
        let vars: HashMap<String, Value<f32>> =
            var_list.into_iter().map(|(k, v)| (k.into(), v)).collect();
        let result = eval(expr.ast(), &vars, &registry).unwrap();
        println!("{} = {:?}", expr_str, result);
    }

    // Vector arithmetic
    println!("\nVector Arithmetic\n{:-<40}", "");

    let expr = Expr::parse("a + b * 2").unwrap();
    let vars: HashMap<String, Value<f32>> =
        [("a".into(), vec2(1.0, 2.0)), ("b".into(), vec2(3.0, 4.0))].into();
    let result = eval(expr.ast(), &vars, &registry).unwrap();
    println!("[1,2] + [3,4] * 2 = {:?}", result);

    // Hadamard (element-wise) product
    let expr = Expr::parse("hadamard(a, b)").unwrap();
    let vars: HashMap<String, Value<f32>> =
        [("a".into(), vec2(2.0, 3.0)), ("b".into(), vec2(4.0, 5.0))].into();
    let result = eval(expr.ast(), &vars, &registry).unwrap();
    println!("hadamard([2,3], [4,5]) = {:?}", result);

    // Reflection
    println!("\nReflection\n{:-<40}", "");

    let expr = Expr::parse("reflect(v, n)").unwrap();
    let vars: HashMap<String, Value<f32>> = [
        ("v".into(), vec2(1.0, -1.0)), // incoming vector
        ("n".into(), vec2(0.0, 1.0)),  // surface normal (up)
    ]
    .into();
    let result = eval(expr.ast(), &vars, &registry).unwrap();
    println!("reflect([1,-1], [0,1]) = {:?}", result);

    // Interpolation
    println!("\nInterpolation\n{:-<40}", "");

    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let expr = Expr::parse("lerp(a, b, t)").unwrap();
        let vars: HashMap<String, Value<f32>> = [
            ("a".into(), vec2(0.0, 0.0)),
            ("b".into(), vec2(10.0, 10.0)),
            ("t".into(), Value::Scalar(t)),
        ]
        .into();
        let result = eval(expr.ast(), &vars, &registry).unwrap();
        println!("lerp([0,0], [10,10], {}) = {:?}", t, result);
    }
}

fn vec2(x: f32, y: f32) -> Value<f32> {
    Value::Vec2([x, y])
}
