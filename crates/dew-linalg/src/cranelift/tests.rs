//! Tests for the Cranelift JIT compiler.

use super::jit::LinalgJit;
use super::types::VarSpec;
use crate::Type;
use dew_core::Expr;

#[test]
fn test_scalar_add() {
    let expr = Expr::parse("a + b").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_scalar(
            expr.ast(),
            &[
                VarSpec::new("a", Type::Scalar),
                VarSpec::new("b", Type::Scalar),
            ],
        )
        .unwrap();
    assert_eq!(func.call(&[3.0, 4.0]), 7.0);
}

#[test]
fn test_dot_vec2() {
    let expr = Expr::parse("dot(a, b)").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_scalar(
            expr.ast(),
            &[VarSpec::new("a", Type::Vec2), VarSpec::new("b", Type::Vec2)],
        )
        .unwrap();
    // dot([1, 2], [3, 4]) = 1*3 + 2*4 = 11
    assert_eq!(func.call(&[1.0, 2.0, 3.0, 4.0]), 11.0);
}

#[test]
fn test_length_vec2() {
    let expr = Expr::parse("length(v)").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_scalar(expr.ast(), &[VarSpec::new("v", Type::Vec2)])
        .unwrap();
    // length([3, 4]) = 5
    assert_eq!(func.call(&[3.0, 4.0]), 5.0);
}

#[test]
fn test_distance_vec2() {
    let expr = Expr::parse("distance(a, b)").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_scalar(
            expr.ast(),
            &[VarSpec::new("a", Type::Vec2), VarSpec::new("b", Type::Vec2)],
        )
        .unwrap();
    // distance([0, 0], [3, 4]) = 5
    assert_eq!(func.call(&[0.0, 0.0, 3.0, 4.0]), 5.0);
}

#[cfg(feature = "3d")]
#[test]
fn test_dot_vec3() {
    let expr = Expr::parse("dot(a, b)").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_scalar(
            expr.ast(),
            &[VarSpec::new("a", Type::Vec3), VarSpec::new("b", Type::Vec3)],
        )
        .unwrap();
    // dot([1, 2, 3], [4, 5, 6]) = 1*4 + 2*5 + 3*6 = 32
    assert_eq!(func.call(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]), 32.0);
}

#[test]
fn test_complex_expression() {
    // length(a - b) should equal distance(a, b)
    let expr = Expr::parse("length(a - b)").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_scalar(
            expr.ast(),
            &[VarSpec::new("a", Type::Vec2), VarSpec::new("b", Type::Vec2)],
        )
        .unwrap();
    assert_eq!(func.call(&[0.0, 0.0, 3.0, 4.0]), 5.0);
}

#[test]
fn test_vec_scalar_mul() {
    let expr = Expr::parse("length(v * 2)").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_scalar(expr.ast(), &[VarSpec::new("v", Type::Vec2)])
        .unwrap();
    // length([3, 4] * 2) = length([6, 8]) = 10
    assert_eq!(func.call(&[3.0, 4.0]), 10.0);
}

#[test]
fn test_compile_vec2_add() {
    // [1, 2] + [3, 4] = [4, 6]
    let expr = Expr::parse("a + b").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_vec2(
            expr.ast(),
            &[VarSpec::new("a", Type::Vec2), VarSpec::new("b", Type::Vec2)],
        )
        .unwrap();
    let [x, y] = func.call(&[1.0, 2.0, 3.0, 4.0]);
    assert_eq!(x, 4.0);
    assert_eq!(y, 6.0);
}

#[test]
fn test_compile_vec2_scalar_mul() {
    // [1, 2] * 3 = [3, 6]
    let expr = Expr::parse("v * 3").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_vec2(expr.ast(), &[VarSpec::new("v", Type::Vec2)])
        .unwrap();
    let [x, y] = func.call(&[1.0, 2.0]);
    assert_eq!(x, 3.0);
    assert_eq!(y, 6.0);
}

#[cfg(feature = "3d")]
#[test]
fn test_compile_vec3_add() {
    // [1, 2, 3] + [4, 5, 6] = [5, 7, 9]
    let expr = Expr::parse("a + b").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_vec3(
            expr.ast(),
            &[VarSpec::new("a", Type::Vec3), VarSpec::new("b", Type::Vec3)],
        )
        .unwrap();
    let [x, y, z] = func.call(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    assert_eq!(x, 5.0);
    assert_eq!(y, 7.0);
    assert_eq!(z, 9.0);
}

#[cfg(feature = "3d")]
#[test]
fn test_compile_vec3_scalar_mul() {
    // [1, 2, 3] * 2 = [2, 4, 6]
    let expr = Expr::parse("v * 2").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_vec3(expr.ast(), &[VarSpec::new("v", Type::Vec3)])
        .unwrap();
    let [x, y, z] = func.call(&[1.0, 2.0, 3.0]);
    assert_eq!(x, 2.0);
    assert_eq!(y, 4.0);
    assert_eq!(z, 6.0);
}

// ========================================================================
// Matrix tests
// ========================================================================

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

#[test]
fn test_mat2_mul_vec2() {
    // Identity matrix times vector
    let expr = Expr::parse("m * v").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_vec2(
            expr.ast(),
            &[VarSpec::new("m", Type::Mat2), VarSpec::new("v", Type::Vec2)],
        )
        .unwrap();
    let [x, y] = func.call(&[1.0, 0.0, 0.0, 1.0, 3.0, 4.0]);
    assert!(approx_eq(x, 3.0));
    assert!(approx_eq(y, 4.0));
}

#[test]
fn test_mat2_mul_vec2_rotation() {
    // 90 degree rotation matrix (column-major): [0, 1, -1, 0]
    let expr = Expr::parse("m * v").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_vec2(
            expr.ast(),
            &[VarSpec::new("m", Type::Mat2), VarSpec::new("v", Type::Vec2)],
        )
        .unwrap();
    let [x, y] = func.call(&[0.0, 1.0, -1.0, 0.0, 1.0, 0.0]);
    assert!(approx_eq(x, 0.0));
    assert!(approx_eq(y, 1.0));
}

#[test]
fn test_vec2_mul_mat2() {
    // Row vector times matrix
    let expr = Expr::parse("v * m").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_vec2(
            expr.ast(),
            &[VarSpec::new("v", Type::Vec2), VarSpec::new("m", Type::Mat2)],
        )
        .unwrap();
    let [x, y] = func.call(&[1.0, 2.0, 1.0, 2.0, 3.0, 4.0]);
    assert!(approx_eq(x, 5.0));
    assert!(approx_eq(y, 11.0));
}

#[test]
fn test_mat2_mul_mat2() {
    // Identity * A = A
    let expr = Expr::parse("a * b").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_mat2(
            expr.ast(),
            &[VarSpec::new("a", Type::Mat2), VarSpec::new("b", Type::Mat2)],
        )
        .unwrap();
    let result = func.call(&[1.0, 0.0, 0.0, 1.0, 1.0, 2.0, 3.0, 4.0]);
    assert!(approx_eq(result[0], 1.0));
    assert!(approx_eq(result[1], 2.0));
    assert!(approx_eq(result[2], 3.0));
    assert!(approx_eq(result[3], 4.0));
}

#[test]
fn test_mat2_scalar_mul() {
    let expr = Expr::parse("m * 2").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_mat2(expr.ast(), &[VarSpec::new("m", Type::Mat2)])
        .unwrap();
    let result = func.call(&[1.0, 2.0, 3.0, 4.0]);
    assert!(approx_eq(result[0], 2.0));
    assert!(approx_eq(result[1], 4.0));
    assert!(approx_eq(result[2], 6.0));
    assert!(approx_eq(result[3], 8.0));
}

#[test]
fn test_mat2_add() {
    let expr = Expr::parse("a + b").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_mat2(
            expr.ast(),
            &[VarSpec::new("a", Type::Mat2), VarSpec::new("b", Type::Mat2)],
        )
        .unwrap();
    let result = func.call(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    assert!(approx_eq(result[0], 6.0));
    assert!(approx_eq(result[1], 8.0));
    assert!(approx_eq(result[2], 10.0));
    assert!(approx_eq(result[3], 12.0));
}

#[test]
fn test_mat2_neg() {
    let expr = Expr::parse("-m").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_mat2(expr.ast(), &[VarSpec::new("m", Type::Mat2)])
        .unwrap();
    let result = func.call(&[1.0, 2.0, 3.0, 4.0]);
    assert!(approx_eq(result[0], -1.0));
    assert!(approx_eq(result[1], -2.0));
    assert!(approx_eq(result[2], -3.0));
    assert!(approx_eq(result[3], -4.0));
}

#[cfg(feature = "3d")]
#[test]
fn test_mat3_mul_vec3() {
    // Identity matrix
    let expr = Expr::parse("m * v").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_vec3(
            expr.ast(),
            &[VarSpec::new("m", Type::Mat3), VarSpec::new("v", Type::Vec3)],
        )
        .unwrap();
    let [x, y, z] = func.call(&[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 4.0]);
    assert!(approx_eq(x, 2.0));
    assert!(approx_eq(y, 3.0));
    assert!(approx_eq(z, 4.0));
}

#[cfg(feature = "3d")]
#[test]
fn test_mat3_scalar_mul() {
    let expr = Expr::parse("m * 2").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_mat3(expr.ast(), &[VarSpec::new("m", Type::Mat3)])
        .unwrap();
    let result = func.call(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
    for i in 0..9 {
        assert!(approx_eq(result[i], (i as f32 + 1.0) * 2.0));
    }
}

#[cfg(feature = "3d")]
#[test]
fn test_mat3_mul_mat3_identity() {
    let expr = Expr::parse("a * b").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_mat3(
            expr.ast(),
            &[VarSpec::new("a", Type::Mat3), VarSpec::new("b", Type::Mat3)],
        )
        .unwrap();
    let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    let mut args = [0.0f32; 18];
    args[..9].copy_from_slice(&identity);
    args[9..].copy_from_slice(&identity);
    let result = func.call(&args);
    for i in 0..9 {
        assert!(approx_eq(result[i], identity[i]));
    }
}

#[cfg(feature = "4d")]
#[test]
fn test_mat4_mul_vec4() {
    let expr = Expr::parse("m * v").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_vec4(
            expr.ast(),
            &[VarSpec::new("m", Type::Mat4), VarSpec::new("v", Type::Vec4)],
        )
        .unwrap();
    let identity = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    let v = [2.0, 3.0, 4.0, 5.0];
    let mut args = [0.0f32; 20];
    args[..16].copy_from_slice(&identity);
    args[16..].copy_from_slice(&v);
    let [x, y, z, w] = func.call(&args);
    assert!(approx_eq(x, 2.0));
    assert!(approx_eq(y, 3.0));
    assert!(approx_eq(z, 4.0));
    assert!(approx_eq(w, 5.0));
}

#[cfg(feature = "4d")]
#[test]
fn test_mat4_scalar_mul() {
    let expr = Expr::parse("m * 2").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_mat4(expr.ast(), &[VarSpec::new("m", Type::Mat4)])
        .unwrap();
    let input: [f32; 16] = std::array::from_fn(|i| i as f32 + 1.0);
    let result = func.call(&input);
    for i in 0..16 {
        assert!(approx_eq(result[i], (i as f32 + 1.0) * 2.0));
    }
}

#[cfg(feature = "4d")]
#[test]
fn test_mat4_mul_mat4_identity() {
    let expr = Expr::parse("a * b").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_mat4(
            expr.ast(),
            &[VarSpec::new("a", Type::Mat4), VarSpec::new("b", Type::Mat4)],
        )
        .unwrap();
    let identity = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    let mut args = [0.0f32; 32];
    args[..16].copy_from_slice(&identity);
    args[16..].copy_from_slice(&identity);
    let result = func.call(&args);
    for i in 0..16 {
        assert!(approx_eq(result[i], identity[i]));
    }
}

#[cfg(feature = "4d")]
#[test]
fn test_mat4_neg() {
    let expr = Expr::parse("-m").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_mat4(expr.ast(), &[VarSpec::new("m", Type::Mat4)])
        .unwrap();
    let input: [f32; 16] = std::array::from_fn(|i| i as f32 + 1.0);
    let result = func.call(&input);
    for i in 0..16 {
        assert!(approx_eq(result[i], -input[i]));
    }
}

#[cfg(feature = "4d")]
#[test]
fn test_vec4_mul_mat4() {
    let expr = Expr::parse("v * m").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_vec4(
            expr.ast(),
            &[VarSpec::new("v", Type::Vec4), VarSpec::new("m", Type::Mat4)],
        )
        .unwrap();
    let identity = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    let v = [2.0, 3.0, 4.0, 5.0];
    let mut args = [0.0f32; 20];
    args[..4].copy_from_slice(&v);
    args[4..].copy_from_slice(&identity);
    let [x, y, z, w] = func.call(&args);
    assert!(approx_eq(x, 2.0));
    assert!(approx_eq(y, 3.0));
    assert!(approx_eq(z, 4.0));
    assert!(approx_eq(w, 5.0));
}

// ========================================================================
// New function tests (cross, normalize, reflect, hadamard, lerp/mix)
// ========================================================================

#[cfg(feature = "3d")]
#[test]
fn test_cross_vec3() {
    let expr = Expr::parse("cross(a, b)").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_vec3(
            expr.ast(),
            &[VarSpec::new("a", Type::Vec3), VarSpec::new("b", Type::Vec3)],
        )
        .unwrap();
    let [x, y, z] = func.call(&[1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
    assert!(approx_eq(x, 0.0));
    assert!(approx_eq(y, 0.0));
    assert!(approx_eq(z, 1.0));
}

#[test]
fn test_normalize_vec2() {
    let expr = Expr::parse("normalize(v)").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_vec2(expr.ast(), &[VarSpec::new("v", Type::Vec2)])
        .unwrap();
    let [x, y] = func.call(&[3.0, 4.0]);
    assert!(approx_eq(x, 0.6));
    assert!(approx_eq(y, 0.8));
}

#[cfg(feature = "3d")]
#[test]
fn test_normalize_vec3() {
    let expr = Expr::parse("normalize(v)").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_vec3(expr.ast(), &[VarSpec::new("v", Type::Vec3)])
        .unwrap();
    let [x, y, z] = func.call(&[0.0, 3.0, 4.0]);
    assert!(approx_eq(x, 0.0));
    assert!(approx_eq(y, 0.6));
    assert!(approx_eq(z, 0.8));
}

#[test]
fn test_reflect_vec2() {
    let expr = Expr::parse("reflect(i, n)").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_vec2(
            expr.ast(),
            &[VarSpec::new("i", Type::Vec2), VarSpec::new("n", Type::Vec2)],
        )
        .unwrap();
    let [x, y] = func.call(&[1.0, -1.0, 0.0, 1.0]);
    assert!(approx_eq(x, 1.0));
    assert!(approx_eq(y, 1.0));
}

#[cfg(feature = "3d")]
#[test]
fn test_reflect_vec3() {
    let expr = Expr::parse("reflect(i, n)").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_vec3(
            expr.ast(),
            &[VarSpec::new("i", Type::Vec3), VarSpec::new("n", Type::Vec3)],
        )
        .unwrap();
    let [x, y, z] = func.call(&[1.0, -1.0, 0.0, 0.0, 1.0, 0.0]);
    assert!(approx_eq(x, 1.0));
    assert!(approx_eq(y, 1.0));
    assert!(approx_eq(z, 0.0));
}

#[test]
fn test_hadamard_vec2() {
    let expr = Expr::parse("hadamard(a, b)").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_vec2(
            expr.ast(),
            &[VarSpec::new("a", Type::Vec2), VarSpec::new("b", Type::Vec2)],
        )
        .unwrap();
    let [x, y] = func.call(&[2.0, 3.0, 4.0, 5.0]);
    assert!(approx_eq(x, 8.0));
    assert!(approx_eq(y, 15.0));
}

#[cfg(feature = "3d")]
#[test]
fn test_hadamard_vec3() {
    let expr = Expr::parse("hadamard(a, b)").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_vec3(
            expr.ast(),
            &[VarSpec::new("a", Type::Vec3), VarSpec::new("b", Type::Vec3)],
        )
        .unwrap();
    let [x, y, z] = func.call(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    assert!(approx_eq(x, 4.0));
    assert!(approx_eq(y, 10.0));
    assert!(approx_eq(z, 18.0));
}

#[test]
fn test_lerp_vec2() {
    let expr = Expr::parse("lerp(a, b, t)").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_vec2(
            expr.ast(),
            &[
                VarSpec::new("a", Type::Vec2),
                VarSpec::new("b", Type::Vec2),
                VarSpec::new("t", Type::Scalar),
            ],
        )
        .unwrap();
    let [x, y] = func.call(&[0.0, 0.0, 10.0, 20.0, 0.5]);
    assert!(approx_eq(x, 5.0));
    assert!(approx_eq(y, 10.0));
}

#[cfg(feature = "3d")]
#[test]
fn test_lerp_vec3() {
    let expr = Expr::parse("lerp(a, b, t)").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_vec3(
            expr.ast(),
            &[
                VarSpec::new("a", Type::Vec3),
                VarSpec::new("b", Type::Vec3),
                VarSpec::new("t", Type::Scalar),
            ],
        )
        .unwrap();
    let [x, y, z] = func.call(&[0.0, 0.0, 0.0, 10.0, 20.0, 30.0, 0.25]);
    assert!(approx_eq(x, 2.5));
    assert!(approx_eq(y, 5.0));
    assert!(approx_eq(z, 7.5));
}

#[test]
fn test_mix_vec2() {
    // mix is alias for lerp
    let expr = Expr::parse("mix(a, b, t)").unwrap();
    let jit = LinalgJit::new().unwrap();
    let func = jit
        .compile_vec2(
            expr.ast(),
            &[
                VarSpec::new("a", Type::Vec2),
                VarSpec::new("b", Type::Vec2),
                VarSpec::new("t", Type::Scalar),
            ],
        )
        .unwrap();
    let [x, y] = func.call(&[0.0, 0.0, 10.0, 20.0, 0.25]);
    assert!(approx_eq(x, 2.5));
    assert!(approx_eq(y, 5.0));
}
