# dew-core

Core expression language for Dew: a parser and AST for small numeric expressions.

## Overview

`dew-core` provides a minimal expression parser that compiles strings like
`"sin(x * pi()) + y"` into an evaluable AST. Variables and functions are
supplied entirely by the caller — nothing is hardcoded — which makes it
suitable for user-facing expression inputs, shader parameter systems, and
dynamic computation pipelines. It has no runtime dependencies beyond
`num-traits` and performs no allocation during evaluation.

Domain crates (`dew-scalar`, `dew-linalg`, `dew-complex`, `dew-quaternion`)
build on top of `dew-core`'s AST to add typed values, function libraries, and
backend code generation.

## Feature flags

| Feature | Description |
|---------|-------------|
| `introspect` | AST introspection (`free_vars`, etc.) — enabled by default |
| `cond` | Conditionals (`if`/`then`/`else`), comparisons (`<`, `<=`, `>`, `>=`, `==`, `!=`), boolean logic (`and`, `or`, `not`) |
| `func` | Function calls via the `ExprFn` trait and `FunctionRegistry` |
| `optimize` | Expression optimization passes (constant folding, algebraic simplification) |

`dew-core` itself has no backend features (wgsl/lua/cranelift/etc.) — those
live in the domain crates, which use `dew-core`'s AST as their common
representation.

## Usage

### Basic arithmetic

```rust
use dew_core::Expr;
use std::collections::HashMap;

let expr = Expr::parse("x * 2 + y").unwrap();

let mut vars = HashMap::new();
vars.insert("x".to_string(), 3.0);
vars.insert("y".to_string(), 1.0);

let value = expr.eval(&vars).unwrap();
assert_eq!(value, 7.0); // 3 * 2 + 1 = 7
```

### Inspecting the AST

```rust
use dew_core::{Expr, Ast, BinOp};

let expr = Expr::parse("a + b * c").unwrap();

match expr.ast() {
    Ast::BinOp(BinOp::Add, left, right) => {
        assert!(matches!(left.as_ref(), Ast::Var(name) if name == "a"));
        assert!(matches!(right.as_ref(), Ast::BinOp(BinOp::Mul, _, _)));
    }
    _ => panic!("unexpected AST structure"),
}
```

### Conditionals (`cond` feature)

```rust
# #[cfg(feature = "cond")]
# {
use dew_core::Expr;
use std::collections::HashMap;

let expr = Expr::parse("if x > 0 then 1 else -1").unwrap();
let mut vars = HashMap::new();
vars.insert("x".to_string(), 5.0);

# #[cfg(not(feature = "func"))]
let value = expr.eval(&vars).unwrap();
# #[cfg(feature = "func")]
# let value = expr.eval(&vars, &dew_core::FunctionRegistry::new()).unwrap();
assert_eq!(value, 1.0);
# }
```

### Custom functions (`func` feature)

```rust
# #[cfg(feature = "func")]
# {
use dew_core::{Expr, ExprFn, FunctionRegistry};

struct Double;
impl ExprFn for Double {
    fn name(&self) -> &str {
        "double"
    }
    fn arg_count(&self) -> usize {
        1
    }
    fn call(&self, args: &[f32]) -> f32 {
        args[0] * 2.0
    }
}

let mut registry = FunctionRegistry::new();
registry.register(Double);

let expr = Expr::parse("double(21)").unwrap();
let vars = std::collections::HashMap::new();
let result = expr.eval(&vars, &registry).unwrap();
assert_eq!(result, 42.0);
# }
```

## Syntax reference

Operators, by precedence (low to high):

| Precedence | Operators | Requires |
|------------|-----------|----------|
| 1 | `if c then a else b` | `cond` |
| 2 | `a or b` (short-circuit) | `cond` |
| 3 | `a and b` (short-circuit) | `cond` |
| 4 | `<` `<=` `>` `>=` `==` `!=` | `cond` |
| 5 | `a + b`, `a - b` | — |
| 6 | `a * b`, `a / b` | — |
| 7 | `a ^ b` (right-associative) | — |
| 8 | `-a`, `not a` | `not` requires `cond` |
| 9 | `(a)`, `f(a, b)` | calls require `func` |

Booleans (with `cond`) are represented as scalars: `0.0` is false, any
non-zero value is true.

## Backends

`dew-core` defines the AST only — it has no backend code generation itself.
The domain crates that depend on it (`dew-scalar`, `dew-linalg`,
`dew-complex`, `dew-quaternion`) compile that AST to WGSL, GLSL, C, CUDA,
HIP, OpenCL, Rust, TokenStream, Lua, and Cranelift.
