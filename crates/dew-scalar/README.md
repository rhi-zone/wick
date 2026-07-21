# dew-scalar

Standard scalar function library for Dew expressions.

## Overview

`dew-scalar` is the foundation for numeric Dew expressions: standard math
functions (`sin`, `cos`, `sqrt`, ...), constants (`pi`, `e`, `tau`), and
evaluation for plain scalar values. Functions are generic over `T: Float`,
so the same registry works for both `f32` and `f64`. It also provides code
generation to WGSL, GLSL, C, CUDA, HIP, OpenCL, Rust, Lua, and Cranelift for
every registered function, plus an integer variant (`register_scalar_int`)
for bitwise-capable types.

## Feature flags

| Feature | Description |
|---------|-------------|
| `wgsl` | WGSL shader code generation |
| `glsl` | GLSL shader code generation |
| `rust` | Rust text codegen |
| `c` | C text codegen (uses `math.h`) |
| `opencl` | OpenCL kernel codegen |
| `cuda` | CUDA kernel codegen |
| `hip` | HIP kernel codegen (AMD ROCm, CUDA-source-compatible) |
| `tokenstream` | Rust `TokenStream` codegen for proc-macros |
| `lua-codegen` | Lua code generation (pure Rust, WASM-safe) |
| `lua` | Lua codegen + execution via `mlua` (implies `lua-codegen`) |
| `cranelift` | Cranelift JIT native compilation |
| `optimize` | Expression optimization passes (enables `dew-core/optimize`) |
| `all` | Enables every backend above |

## Usage

### Parsing and evaluating

```rust
use dew_core::Expr;
use dew_scalar::{eval, scalar_registry};
use std::collections::HashMap;

let expr = Expr::parse("sin(x * pi()) + 1").unwrap();
let vars: HashMap<String, f32> = [("x".into(), 0.5)].into();
let result = eval(expr.ast(), &vars, &scalar_registry()).unwrap();
assert!((result - 2.0).abs() < 0.001); // sin(0.5 * pi) + 1 = 2
```

### More functions

```rust
use dew_core::Expr;
use dew_scalar::{eval, scalar_registry};
use std::collections::HashMap;

let registry = scalar_registry::<f32>();
for (expr_str, vars) in [
    ("sqrt(16)", HashMap::new()),
    ("lerp(0, 100, 0.25)", HashMap::new()),
    ("clamp(x, 0, 1)", [("x".into(), 1.5)].into()),
    ("smoothstep(0, 1, t)", [("t".into(), 0.5)].into()),
] {
    let expr = Expr::parse(expr_str).unwrap();
    let result = eval(expr.ast(), &vars, &registry).unwrap();
    println!("{} = {}", expr_str, result);
}
```

### Compiling to WGSL

```rust
# #[cfg(feature = "wgsl")]
# {
use dew_core::Expr;
use dew_scalar::wgsl::{emit_wgsl, emit_wgsl_fn};

let expr = Expr::parse("if x > 0 then sqrt(x) else 0").unwrap();
let wgsl = emit_wgsl(expr.ast()).unwrap();
println!("{}", wgsl.code);

// Or emit a full named function:
let expr = Expr::parse("clamp(x * 2, 0, 1)").unwrap();
let func = emit_wgsl_fn("saturate_double", expr.ast(), &["x"]).unwrap();
println!("{}", func);
# }
```

### Compiling to native code with Cranelift

```rust
# #[cfg(feature = "cranelift")]
# {
use dew_core::Expr;
use dew_scalar::cranelift::ScalarJit;

let expr = Expr::parse("x * x + 1").unwrap();
let compiled = ScalarJit::new().unwrap().compile(expr.ast(), &["x"]).unwrap();
assert_eq!(compiled.call(&[3.0]), 10.0);
# }
```

## Available functions

**Constants:** `pi()`, `e()`, `tau()`

**Trigonometric:** `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2(y, x)`, `sinh`, `cosh`, `tanh`

**Exponential/logarithmic:** `exp`, `exp2`, `log`/`ln`, `log2`, `log10`, `pow(x, y)`, `sqrt`, `inversesqrt`

**Common math:** `abs`, `sign`, `floor`, `ceil`, `round`, `trunc`, `fract`, `min`, `max`, `clamp(x, lo, hi)`, `saturate`

**Interpolation:** `lerp(a, b, t)`, `mix` (alias), `step(edge, x)`, `smoothstep(e0, e1, x)`, `inverse_lerp(a, b, v)`, `remap(x, i0, i1, o0, o1)`

Custom functions can be added by implementing the `ScalarFn` trait and
registering them alongside `register_scalar`.

## Backends

WGSL, GLSL, C, OpenCL, CUDA, HIP, Rust (text and `TokenStream`), Lua
(codegen and `mlua` execution), and Cranelift JIT — each behind its own
feature flag.
