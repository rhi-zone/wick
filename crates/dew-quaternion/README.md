# dew-quaternion

Quaternion support for Dew expressions.

## Overview

`dew-quaternion` adds a quaternion value type and function library
(construction, conjugate/inverse, `slerp`/`lerp`, axis-angle conversion,
vector rotation) on top of `dew-core`'s AST, for use in 3D rotation
expressions — camera controllers, skeletal animation, orientation blending.
Component order is `[x, y, z, w]` (scalar last), matching GLM, glTF, and
Unity's internal representation.

## Feature flags

| Feature | Description |
|---------|-------------|
| `wgsl` | WGSL shader code generation |
| `glsl` | GLSL shader code generation |
| `rust` | Rust text codegen |
| `c` | C text codegen |
| `opencl` | OpenCL kernel codegen |
| `cuda` | CUDA kernel codegen |
| `hip` | HIP kernel codegen (AMD ROCm) |
| `tokenstream` | Rust `TokenStream` codegen for proc-macros |
| `lua-codegen` | Lua code generation (pure Rust, WASM-safe) |
| `lua` | Lua codegen + execution via `mlua` (implies `lua-codegen`) |
| `cranelift` | Cranelift JIT native compilation |
| `optimize` | Expression optimization passes (enables `dew-core/optimize`) |
| `all` | Enables every feature above |

## Types

| Type | Description |
|------|-------------|
| `Scalar` | Real number |
| `Vec3` | 3D vector `[x, y, z]` |
| `Quaternion` | Quaternion `[x, y, z, w]` |

## Usage

### Building a rotation and rotating a vector

```rust
use dew_core::Expr;
use dew_quaternion::{Value, eval, quaternion_registry};
use std::collections::HashMap;

let registry = quaternion_registry::<f32>();
let pi = std::f32::consts::PI;

// 90 degree rotation around Y axis should turn [1,0,0] into [0,0,-1]
let expr = Expr::parse("rotate(v, axis_angle(axis, angle))").unwrap();
let vars: HashMap<String, Value<f32>> = [
    ("axis".into(), Value::Vec3([0.0, 1.0, 0.0])),
    ("angle".into(), Value::Scalar(pi / 2.0)),
    ("v".into(), Value::Vec3([1.0, 0.0, 0.0])),
]
.into();
let result = eval(expr.ast(), &vars, &registry).unwrap();
println!("{:?}", result); // ~[0, 0, -1]
```

### Spherical interpolation (slerp)

```rust
use dew_core::Expr;
use dew_quaternion::{Value, eval, quaternion_registry};
use std::collections::HashMap;

let registry = quaternion_registry::<f32>();
let expr = Expr::parse("slerp(q1, q2, t)").unwrap();
let vars: HashMap<String, Value<f32>> = [
    ("q1".into(), Value::Quaternion([0.0, 0.0, 0.0, 1.0])), // identity
    ("q2".into(), Value::Quaternion([0.0, 1.0, 0.0, 0.0])), // 180 deg around Y
    ("t".into(), Value::Scalar(0.5)),
]
.into();
let result = eval(expr.ast(), &vars, &registry).unwrap();
println!("{:?}", result);
```

### Compiling to WGSL

```rust
# #[cfg(feature = "wgsl")]
# {
use dew_core::Expr;
use dew_quaternion::wgsl::emit_wgsl;
use dew_quaternion::Type;
use std::collections::HashMap;

let expr = Expr::parse("normalize(q)").unwrap();
let var_types: HashMap<String, Type> = [("q".to_string(), Type::Quaternion)].into();

let wgsl = emit_wgsl(expr.ast(), &var_types).unwrap();
println!("{}", wgsl.code);
# }
```

## Functions

`vec3(x, y, z)`, `quat(x, y, z, w)`, `conj(q)`, `length(q)`, `normalize(q)`,
`inverse(q)`, `dot(q1, q2)`, `lerp(q1, q2, t)`, `slerp(q1, q2, t)`,
`axis_angle(axis, theta)`, `rotate(v, q)`.

## Operators

`q1 * q2` (quaternion multiplication), `q * scalar`, `q1 + q2`, `q1 - q2`, `-q`.

## Backends

WGSL, GLSL, C, OpenCL, CUDA, HIP, Rust (text and `TokenStream`), Lua
(codegen and `mlua` execution), and Cranelift JIT — each behind its own
feature flag.
