# dew-cond

Backend helpers for conditionals, comparisons, and boolean logic in Dew expressions.

## Overview

`dew-cond` provides shared code-generation utilities for the `Compare`,
`And`, `Or`, and `If` AST nodes that `dew-core` defines under its `cond`
feature. Rather than every domain crate (`dew-scalar`, `dew-linalg`,
`dew-complex`, `dew-quaternion`) reimplementing backend-specific conditional
logic, they delegate to these helpers when emitting code for a given target.

Each backend module (`wgsl`, `glsl`, `rust`, `c`, `opencl`, `cuda`, `hip`,
`lua`, `cranelift`) exposes the same function set with backend-appropriate
implementations:

| Function | Description |
|----------|-------------|
| `emit_compare` | Comparison operators (`<`, `<=`, `>`, `>=`, `==`, `!=`) |
| `emit_and` | Logical AND |
| `emit_or` | Logical OR |
| `emit_not` | Logical NOT |
| `emit_if` | Conditional expression (if/then/else) |
| `scalar_to_bool` | Convert a numeric value to a backend-native boolean |
| `bool_to_scalar` | Convert a backend-native boolean back to a numeric value |

Booleans in Dew are represented as scalars (`0.0` = false, non-zero = true);
`scalar_to_bool`/`bool_to_scalar` bridge that representation to whatever a
given backend natively supports (WGSL `bool`, Lua's truthy values, etc.).

## Feature flags

| Feature | Description |
|---------|-------------|
| `wgsl` | WGSL code generation helpers |
| `glsl` | GLSL code generation helpers |
| `rust` | Rust text codegen helpers |
| `c` | C text codegen helpers |
| `opencl` | OpenCL kernel codegen helpers |
| `cuda` | CUDA kernel codegen helpers |
| `hip` | HIP (AMD ROCm) kernel codegen helpers |
| `tokenstream` | Rust `TokenStream` codegen for proc-macros (pulls in `quote`/`proc-macro2`) |
| `lua-codegen` | Pure-Rust Lua code generation (no native deps, WASM-safe) |
| `lua` | Alias for `lua-codegen` |
| `cranelift` | Cranelift JIT compilation helpers |
| `all` | Enables every backend above |

## Usage

### WGSL backend

```rust
# #[cfg(feature = "wgsl")]
# {
use dew_cond::{wgsl, CompareOp};

// Comparison
let code = wgsl::emit_compare(CompareOp::Lt, "a", "b");
assert_eq!(code, "(a < b)");

// Conditional (uses WGSL's select function)
let code = wgsl::emit_if("cond", "then_val", "else_val");
assert_eq!(code, "select(else_val, then_val, cond)");

// Boolean logic
let code = wgsl::emit_and("x > 0.0", "y > 0.0");
assert_eq!(code, "(x > 0.0 && y > 0.0)");
# }
```

### Lua backend

```rust
# #[cfg(feature = "lua-codegen")]
# {
use dew_cond::{lua, CompareOp};

// Comparison (Lua uses ~= for not-equal)
let code = lua::emit_compare(CompareOp::Ne, "a", "b");
assert_eq!(code, "(a ~= b)");

// Conditional (uses Lua's and/or idiom)
let code = lua::emit_if("cond", "then_val", "else_val");
assert_eq!(code, "(cond and then_val or else_val)");

// Boolean logic
let code = lua::emit_not("flag");
assert_eq!(code, "(not flag)");
# }
```

### Boolean conversion

```rust
# #[cfg(feature = "wgsl")]
# {
use dew_cond::wgsl;

let bool_expr = wgsl::scalar_to_bool("x");
assert_eq!(bool_expr, "(x != 0.0)");

let scalar_expr = wgsl::bool_to_scalar("flag");
assert_eq!(scalar_expr, "select(0.0, 1.0, flag)");
# }
```

This crate is a low-level building block — most applications use it
indirectly through a domain crate's own backend modules (e.g.
`dew_scalar::wgsl`) rather than calling it directly.

## Backends

WGSL, GLSL, C, OpenCL, CUDA, HIP, Rust text, Rust `TokenStream`, Lua, and
Cranelift, each behind its own feature flag.
