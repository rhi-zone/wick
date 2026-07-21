# TokenStream Backend

Generate `proc_macro2::TokenStream` from dew expressions for use in Rust proc-macros.

## Enable

```toml
dew-scalar = { version = "0.1", features = ["tokenstream"] }
dew-linalg = { version = "0.1", features = ["tokenstream"] }
```

## dew-scalar

### Generate TokenStream

```rust
use dew_core::Expr;
use dew_scalar::tokenstream::emit_tokenstream;

let expr = Expr::parse("sin(x) + cos(y)").unwrap();
let tokens = emit_tokenstream(expr.ast()).unwrap();

// tokens is a proc_macro2::TokenStream
// Output equivalent: (x.sin() + y.cos())
```

## dew-linalg

### Generate with Types (glam-compatible)

```rust
use dew_core::Expr;
use dew_linalg::tokenstream::emit_tokenstream;
use dew_linalg::Type;
use std::collections::HashMap;

let expr = Expr::parse("normalize(v) * 2.0").unwrap();

let mut var_types: HashMap<String, Type> = HashMap::new();
var_types.insert("v".to_string(), Type::Vec3);

let result = emit_tokenstream(expr.ast(), &var_types).unwrap();

// result.code is a TokenStream
// result.typ is Type::Vec3
// Output equivalent: (v.normalize() * 2.0)
```

## glam Compatibility

The linalg backend emits code compatible with the [glam](https://docs.rs/glam) math library:

| dew Type | glam Type |
|----------|-----------|
| Scalar | `f32` |
| Vec2 | `Vec2` |
| Vec3 | `Vec3` |
| Vec4 | `Vec4` |
| Mat2 | `Mat2` |
| Mat3 | `Mat3` |
| Mat4 | `Mat4` |

### Vector Constructors

```rust
// dew: vec2(x, y)
// Output: Vec2::new(x, y)

// dew: vec3(x, y, z)
// Output: Vec3::new(x, y, z)
```

### Method Syntax

TokenStream output uses Rust method syntax:

```rust
// dew: dot(a, b)
// Output: a.dot(b)

// dew: normalize(v)
// Output: v.normalize()

// dew: lerp(a, b, t)
// Output: a.lerp(b, t)
```

## Function Mapping

| dew | TokenStream Output |
|-----|-------------------|
| `sin(x)` | `x.sin()` |
| `cos(x)` | `x.cos()` |
| `sqrt(x)` | `x.sqrt()` |
| `abs(x)` | `x.abs()` |
| `floor(x)` | `x.floor()` |
| `x ^ y` | `x.powf(y)` |
| `min(a, b)` | `a.min(b)` |
| `max(a, b)` | `a.max(b)` |
| `clamp(x, lo, hi)` | `x.clamp(lo, hi)` |
| `pi()` | `::std::f32::consts::PI` |
| `e()` | `::std::f32::consts::E` |
| `tau()` | `::std::f32::consts::TAU` |

## Use Case: Proc-Macro Derive

The primary use case is generating code at compile time in procedural macros:

```rust
use proc_macro::TokenStream;
use quote::quote;
use dew_core::Expr;
use dew_scalar::tokenstream::emit_tokenstream;

#[proc_macro]
pub fn dew(input: TokenStream) -> TokenStream {
    let expr_str = input.to_string();
    let expr = Expr::parse(&expr_str).expect("invalid dew expression");
    let tokens = emit_tokenstream(expr.ast()).expect("codegen failed");

    quote! {
        fn generated(x: f32, y: f32) -> f32 {
            #tokens
        }
    }.into()
}
```

## Comparison with Rust Text Backend

| Feature | TokenStream | Rust text |
|---------|-------------|-----------|
| Output | `proc_macro2::TokenStream` | `String` |
| Use case | Proc-macros, compile-time | Debug, docs, runtime codegen |
| Parsing | Direct use in macros | Requires re-parse with `syn` |
| Dependencies | `proc_macro2`, `quote` | None |

## Type Inference

The linalg backend performs full type inference:

```rust
// Matrix-vector multiplication infers result type
let result = emit_tokenstream(
    Expr::parse("m * v").unwrap().ast(),
    &[("m", Type::Mat3), ("v", Type::Vec3)].into_iter().collect()
).unwrap();

assert_eq!(result.typ, Type::Vec3);
```

## See Also

- [Rust Backend](./rust.md) - Text-based Rust codegen for debug/docs
