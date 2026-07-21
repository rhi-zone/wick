//! Quaternion example: 3D rotations.
//!
//! Run with: cargo run --example quaternion

use dew_core::Expr;
use dew_quaternion::{Value, eval, quaternion_registry};
use std::collections::HashMap;

fn main() {
    let registry = quaternion_registry();
    let pi = std::f32::consts::PI;

    // Create rotation from axis-angle
    println!("Axis-Angle to Quaternion\n{:-<40}", "");

    let expr = Expr::parse("axis_angle(axis, angle)").unwrap();
    let vars: HashMap<String, Value<f32>> = [
        ("axis".into(), vec3(0.0, 1.0, 0.0)),      // Y axis
        ("angle".into(), Value::Scalar(pi / 2.0)), // 90 degrees
    ]
    .into();
    let result = eval(expr.ast(), &vars, &registry).unwrap();
    println!("90° rotation around Y axis: {:?}", result);

    // Rotate a vector
    println!("\nRotate Vector\n{:-<40}", "");

    // 90° rotation around Y axis should turn [1,0,0] into [0,0,-1]
    let expr = Expr::parse("rotate(v, q)").unwrap();
    let q = quat_axis_angle([0.0, 1.0, 0.0], pi / 2.0);
    let vars: HashMap<String, Value<f32>> =
        [("q".into(), q), ("v".into(), vec3(1.0, 0.0, 0.0))].into();
    let result = eval(expr.ast(), &vars, &registry).unwrap();
    println!("Rotate [1,0,0] by 90° around Y: {:?}", result);
    println!("  Expected: [0, 0, -1]");

    // Quaternion operations
    println!("\nQuaternion Operations\n{:-<40}", "");

    let q = quat(0.0, 0.0, 0.707, 0.707); // ~90° around Z
    let vars: HashMap<String, Value<f32>> = [("q".into(), q)].into();

    for func in ["length(q)", "normalize(q)", "conj(q)", "inverse(q)"] {
        let expr = Expr::parse(func).unwrap();
        let result = eval(expr.ast(), &vars, &registry).unwrap();
        println!("{} = {:?}", func, result);
    }

    // Quaternion multiplication (combining rotations)
    println!("\nCombining Rotations\n{:-<40}", "");

    let q1 = quat_axis_angle([0.0, 1.0, 0.0], pi / 2.0); // 90° around Y
    let q2 = quat_axis_angle([1.0, 0.0, 0.0], pi / 2.0); // 90° around X
    let expr = Expr::parse("q1 * q2").unwrap();
    let vars: HashMap<String, Value<f32>> = [("q1".into(), q1), ("q2".into(), q2)].into();
    let result = eval(expr.ast(), &vars, &registry).unwrap();
    println!("Rot_Y(90°) * Rot_X(90°) = {:?}", result);

    // Spherical interpolation (slerp)
    println!("\nSpherical Interpolation (Slerp)\n{:-<40}", "");

    let q1 = quat(0.0, 0.0, 0.0, 1.0); // identity
    let q2 = quat_axis_angle([0.0, 1.0, 0.0], pi); // 180° around Y

    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let expr = Expr::parse("slerp(q1, q2, t)").unwrap();
        let vars: HashMap<String, Value<f32>> = [
            ("q1".into(), q1.clone()),
            ("q2".into(), q2.clone()),
            ("t".into(), Value::Scalar(t)),
        ]
        .into();
        let result = eval(expr.ast(), &vars, &registry).unwrap();
        println!("slerp(identity, 180°Y, {:.2}) = {:?}", t, result);
    }
}

fn vec3(x: f32, y: f32, z: f32) -> Value<f32> {
    Value::Vec3([x, y, z])
}

fn quat(x: f32, y: f32, z: f32, w: f32) -> Value<f32> {
    Value::Quaternion([x, y, z, w])
}

/// Create quaternion from axis-angle (axis should be normalized)
fn quat_axis_angle(axis: [f32; 3], angle: f32) -> Value<f32> {
    let half = angle / 2.0;
    let s = half.sin();
    let c = half.cos();
    Value::Quaternion([axis[0] * s, axis[1] * s, axis[2] * s, c])
}
