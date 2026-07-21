# dew

Umbrella crate for the Dew expression language — one combined `Value` type across all domains.

## Overview

Dew is a minimal expression language: small, ephemeral, perfectly formed —
like a droplet condensed from logic. You parse a string like
`"sin(x * pi()) + cos(y)"` once with `dew-core`, then evaluate it or emit it
to a backend (WGSL, GLSL, Lua, Cranelift, C, CUDA, ...) using whichever
domain crate understands the values involved.

The name follows from that image: expressions condense down to a `.dew`
file — a small droplet of logic rather than a sprawling program.

The `dew` crate is the entry point for most users. It re-exports the domain
crates (`dew-scalar`, `dew-linalg`, `dew-complex`, `dew-quaternion`) behind
feature flags and provides a single combined `Value<T>` enum that
implements every domain's value trait at once — so one expression can freely
mix scalars, vectors, complex numbers, and quaternions without you having to
pick a single domain crate's value type up front.

## Architecture

```
dew-core               # Syntax only: AST, parsing
    |
    +-- dew-cond       # Conditional backend helpers (shared by domain crates)
    |
    +-- dew-scalar     # Scalar domain: f32/f64 math functions
    |
    +-- dew-linalg     # Linalg domain: Vec2/3/4, Mat2/3/4
    |
    +-- dew-complex    # Complex numbers: [re, im]
    |
    +-- dew-quaternion # Quaternions: [x, y, z, w]
    |
    +-- dew            # This crate: combined Value<T> over all of the above
```

Domain crates are independent and can be used directly; `dew` exists purely
to compose them behind one `Value` type and one set of feature flags.

## Feature flags

Domain selection (all enabled by default):

| Feature | Description |
|---------|--------------|
| `scalar` | Scalar math functions (pulls in `dew-scalar`) |
| `linalg` | Vectors and matrices (pulls in `dew-linalg`, with `4d`) |
| `complex` | Complex numbers (pulls in `dew-complex`) |
| `quaternion` | Quaternions (pulls in `dew-quaternion`) |

Backend selection (propagated to every enabled domain crate):

| Feature | Description |
|---------|--------------|
| `wgsl` | WGSL shader code generation |
| `glsl` | GLSL shader code generation |
| `rust` | Rust text codegen |
| `c` | C text codegen |
| `opencl` | OpenCL kernel codegen |
| `cuda` | CUDA kernel codegen |
| `hip` | HIP kernel codegen (AMD ROCm) |
| `tokenstream` | Rust `TokenStream` codegen for proc-macros |
| `lua` | Lua code generation + `mlua` execution |
| `cranelift` | Cranelift JIT native compilation |
| `optimize` | Expression optimization passes |

Picking backends: enable only what you need. For example, a build that only
targets WGSL shaders and skips native JIT support:

```toml
[dependencies]
dew = { version = "0.1", default-features = false, features = ["scalar", "linalg", "wgsl"] }
```

## Usage

### The combined `Value` type

```rust
use dew::Value;

let scalar: Value<f32> = Value::Scalar(1.0);
let vec3: Value<f32> = Value::Vec3([1.0, 2.0, 3.0]);
let complex: Value<f32> = Value::Complex([1.0, 2.0]);
let quat: Value<f32> = Value::Quaternion([0.0, 0.0, 0.0, 1.0]);
```

### Using it with a domain crate's `eval`

`Value<T>` implements each domain's value trait (`LinalgValue`,
`ComplexValue`, `QuaternionValue`), so it plugs directly into that domain's
`eval` function and function registry:

```rust
# #[cfg(feature = "linalg")]
# {
use dew::Value;
use dew_core::Expr;
use dew_linalg::{eval, FunctionRegistry, register_linalg};
use std::collections::HashMap;

let expr = Expr::parse("dot(a, b)").unwrap();
let vars: HashMap<String, Value<f32>> = [
    ("a".into(), Value::Vec3([1.0, 0.0, 0.0])),
    ("b".into(), Value::Vec3([1.0, 0.0, 0.0])),
]
.into();

// Build a registry over the combined Value type
let mut registry = FunctionRegistry::new();
register_linalg(&mut registry);

let result = eval(expr.ast(), &vars, &registry).unwrap();
assert_eq!(result, Value::Scalar(1.0));
# }
```

### Mixing domains in one expression

Because `Value<T>` implements every domain trait, an expression that reads
a `Vec3` and a `Quaternion` and returns another `Vec3` (e.g. rotating a
vector) works the same way through `dew_quaternion::eval`:

```rust
# #[cfg(feature = "quaternion")]
# {
use dew::Value;
use dew_core::Expr;
use dew_quaternion::{eval, FunctionRegistry, register_quaternion};
use std::collections::HashMap;

let mut registry = FunctionRegistry::new();
register_quaternion(&mut registry);

let expr = Expr::parse("rotate(v, q)").unwrap();
let vars: HashMap<String, Value<f32>> = [
    ("v".into(), Value::Vec3([1.0, 0.0, 0.0])),
    ("q".into(), Value::Quaternion([0.0, 0.0, 0.0, 1.0])), // identity
]
.into();

let result = eval(expr.ast(), &vars, &registry).unwrap();
assert_eq!(result, Value::Vec3([1.0, 0.0, 0.0]));
# }
```

### Compiling to a backend

Code generation is still done per-domain (a WGSL emitter needs to know
whether a variable is a `vec3` or a `complex`), so use the relevant domain
crate's backend module directly, e.g. `dew_linalg::wgsl::emit_wgsl` or
`dew_scalar::wgsl::emit_wgsl`. See the `dew-scalar`, `dew-linalg`,
`dew-complex`, and `dew-quaternion` READMEs for backend-specific examples.

## When to use `dew` vs. a domain crate directly

Use `dew` when your application needs to accept expressions that may touch
more than one domain (e.g. a general-purpose shader parameter system) and
you want a single `Value` type to carry them. Use a domain crate directly
(`dew-scalar`, `dew-linalg`, `dew-complex`, `dew-quaternion`) when you know
in advance which domain your expressions belong to — it avoids pulling in
the others and keeps the value type minimal.

## Backends

WGSL, GLSL, C, OpenCL, CUDA, HIP, Rust (text and `TokenStream`), Lua
(codegen and `mlua` execution), and Cranelift JIT — each behind its own
feature flag, propagated to every enabled domain crate.
