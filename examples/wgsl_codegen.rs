//! WGSL code generation example.
//!
//! Run with: cargo run --example wgsl_codegen --features wgsl

use dew_core::Expr;
use dew_scalar::wgsl::{emit_wgsl, emit_wgsl_fn};

fn main() {
    let expressions = [
        "sin(x) + cos(y)",
        "lerp(0, 1, t)",
        "smoothstep(0, 1, t)",
        "if x > 0 then sqrt(x) else 0",
        "clamp(x * 2, 0, 1)",
    ];

    println!("WGSL Code Generation\n");
    println!("{:-<60}", "");

    for expr_str in expressions {
        let expr = Expr::parse(expr_str).unwrap();
        match emit_wgsl(expr.ast()) {
            Ok(wgsl) => {
                println!("Expression: {}", expr_str);
                println!("WGSL:       {}", wgsl.code);
                println!();
            }
            Err(e) => {
                println!("Expression: {} -> Error: {:?}", expr_str, e);
            }
        }
    }

    // Generate a complete WGSL function
    println!("{:-<60}", "");
    println!("\nComplete WGSL function example:\n");

    let expr =
        Expr::parse("if t < 0.5 then lerp(a, b, t * 2) else lerp(b, c, (t - 0.5) * 2)").unwrap();

    match emit_wgsl_fn("ease_through", expr.ast(), &["a", "b", "c", "t"]) {
        Ok(func) => println!("{}", func),
        Err(e) => println!("Error: {:?}", e),
    }
}
