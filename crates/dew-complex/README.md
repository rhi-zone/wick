# dew-complex

Complex number support for Dew expressions.

## Overview

`dew-complex` adds a complex-number value type and function library
(construction, conjugate, modulus/argument, complex `exp`/`log`/`sqrt`/`pow`,
and the arithmetic operators) on top of `dew-core`'s AST. It's aimed at
signal processing, 2D rotations, and general complex arithmetic where
expressions need to move seamlessly between real and complex operands.

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
| `Complex` | Complex number `[re, im]` |

## Usage

### Complex arithmetic

```rust
use dew_core::Expr;
use dew_complex::{Value, eval, complex_registry};
use std::collections::HashMap;

let registry = complex_registry::<f32>();

let expr = Expr::parse("a * b").unwrap();
let vars: HashMap<String, Value<f32>> = [
    ("a".into(), Value::Complex([1.0, 2.0])), // 1 + 2i
    ("b".into(), Value::Complex([3.0, 4.0])), // 3 + 4i
]
.into();
let result = eval(expr.ast(), &vars, &registry).unwrap();
assert_eq!(result, Value::Complex([-5.0, 10.0])); // (1+2i)(3+4i) = -5 + 10i
```

### Euler's formula

```rust
use dew_core::Expr;
use dew_complex::{Value, eval, complex_registry};
use std::collections::HashMap;

let registry = complex_registry::<f32>();
let pi = std::f32::consts::PI;

let expr = Expr::parse("exp(z)").unwrap();
let vars: HashMap<String, Value<f32>> = [("z".into(), Value::Complex([0.0, pi]))].into();
let result = eval(expr.ast(), &vars, &registry).unwrap();
// exp(i*pi) = -1 + 0i
println!("{:?}", result);
```

### Compiling to WGSL

```rust
# #[cfg(feature = "wgsl")]
# {
use dew_core::Expr;
use dew_complex::wgsl::emit_wgsl;
use dew_complex::Type;
use std::collections::HashMap;

let expr = Expr::parse("conj(z) * z").unwrap();
let var_types: HashMap<String, Type> = [("z".to_string(), Type::Complex)].into();

let wgsl = emit_wgsl(expr.ast(), &var_types).unwrap();
println!("{}", wgsl.code);
# }
```

## Functions

`complex(re, im)`, `polar(r, theta)`, `re(z)`, `im(z)`, `conj(z)`, `abs(z)`,
`arg(z)`, `norm(z)`, `exp(z)`, `log(z)`, `sqrt(z)`, `pow(z, n)`.

## Operators

`z1 + z2`, `z1 - z2`, `z1 * z2`, `z1 / z2`, `z * scalar`, `-z`.

## Backends

WGSL, GLSL, C, OpenCL, CUDA, HIP, Rust (text and `TokenStream`), Lua
(codegen and `mlua` execution), and Cranelift JIT — each behind its own
feature flag.
