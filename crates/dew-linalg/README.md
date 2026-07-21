# dew-linalg

Linear algebra types and operations for Dew expressions.

## Overview

`dew-linalg` adds vector and matrix types (`Vec2`, `Vec3`, `Vec4`, `Mat2`,
`Mat3`, `Mat4`) on top of `dew-core`'s AST, with arithmetic operators and a
function library (`dot`, `cross`, `length`, `normalize`, `reflect`, ...)
that type-check and propagate shapes during evaluation or code generation.
It's the crate to reach for when expressions need to manipulate points,
directions, or transforms rather than plain scalars — e.g. shader uniforms,
physics expressions, or procedural geometry.

## Feature flags

| Feature | Description |
|---------|-------------|
| `3d` | `Vec3`, `Mat3` (enabled by default) |
| `4d` | `Vec4`, `Mat4` (implies `3d`) |
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
| `Scalar` | Single `f32`/`f64` value |
| `Vec2` | 2D vector `[x, y]` |
| `Vec3` | 3D vector `[x, y, z]` (`3d`) |
| `Vec4` | 4D vector `[x, y, z, w]` (`4d`) |
| `Mat2` | 2x2 matrix, column-major |
| `Mat3` | 3x3 matrix, column-major (`3d`) |
| `Mat4` | 4x4 matrix, column-major (`4d`) |

## Usage

### Evaluating vector expressions

```rust
use dew_core::Expr;
use dew_linalg::{Value, eval, linalg_registry};
use std::collections::HashMap;

let expr = Expr::parse("dot(a, b)").unwrap();
let vars: HashMap<String, Value<f32>> = [
    ("a".into(), Value::Vec2([1.0, 0.0])),
    ("b".into(), Value::Vec2([0.0, 1.0])),
]
.into();

let result = eval(expr.ast(), &vars, &linalg_registry()).unwrap();
assert_eq!(result, Value::Scalar(0.0)); // perpendicular vectors
```

### Vector arithmetic and helper functions

```rust
use dew_core::Expr;
use dew_linalg::{Value, eval, linalg_registry};
use std::collections::HashMap;

let registry = linalg_registry::<f32>();

let expr = Expr::parse("a + b * 2").unwrap();
let vars: HashMap<String, Value<f32>> =
    [("a".into(), Value::Vec2([1.0, 2.0])), ("b".into(), Value::Vec2([3.0, 4.0]))].into();
let result = eval(expr.ast(), &vars, &registry).unwrap();
println!("[1,2] + [3,4] * 2 = {:?}", result);

let expr = Expr::parse("reflect(v, n)").unwrap();
let vars: HashMap<String, Value<f32>> = [
    ("v".into(), Value::Vec2([1.0, -1.0])), // incoming vector
    ("n".into(), Value::Vec2([0.0, 1.0])),  // surface normal (up)
]
.into();
let result = eval(expr.ast(), &vars, &registry).unwrap();
println!("reflect([1,-1], [0,1]) = {:?}", result);
```

### Compiling to WGSL

```rust
# #[cfg(feature = "wgsl")]
# {
use dew_core::Expr;
use dew_linalg::wgsl::emit_wgsl;
use dew_linalg::Type;
use std::collections::HashMap;

let expr = Expr::parse("normalize(a + b)").unwrap();
let var_types: HashMap<String, Type> =
    [("a".to_string(), Type::Vec3), ("b".to_string(), Type::Vec3)].into();

let wgsl = emit_wgsl(expr.ast(), &var_types).unwrap();
println!("{}", wgsl.code); // e.g. normalize(a + b)
# }
```

## Functions

`dot(a, b)`, `cross(a, b)` (3d), `length(v)`, `normalize(v)`, `distance(a, b)`,
`reflect(v, n)`, `hadamard(a, b)`, `lerp(a, b, t)`/`mix(a, b, t)`.

## Operators

Component-wise `+`, `-`, `-v` (negation); `vec * scalar` / `scalar * vec`;
`mat * vec` and `mat * mat`.

## Composability

The `LinalgValue` trait lets you define your own combined value type that
implements linalg operations alongside another domain (complex, quaternion),
so a single `Value` enum can flow through multiple domain-crate evaluators.
The `dew` umbrella crate provides one such combined `Value` type out of the box.

## Backends

WGSL, GLSL, C, OpenCL, CUDA, HIP, Rust (text and `TokenStream`), Lua
(codegen and `mlua` execution), and Cranelift JIT — each behind its own
feature flag.
