# API Reference

Complete reference for all Dew operators and functions.

## Operator Precedence

Lower precedence numbers bind tighter (evaluate first).

| Precedence | Operators | Associativity | Description |
|------------|-----------|---------------|-------------|
| 4 (highest) | `^` | Right | Power |
| 3 | `-` (unary), `~` | Right | Negation, bitwise NOT |
| 2 | `*`, `/`, `%` | Left | Multiplication, division, modulo |
| 1 | `+`, `-` | Left | Addition, subtraction |
| 0 | `<<`, `>>` | Left | Bit shift |
| -1 | `&` | Left | Bitwise AND |
| -2 | `\|` | Left | Bitwise OR |
| -3 | `<`, `<=`, `>`, `>=`, `==`, `!=` | Left | Comparison |
| -4 | `not` | Right | Logical NOT |
| -5 | `and` | Left | Logical AND (short-circuit) |
| -6 | `or` | Left | Logical OR (short-circuit) |

**Examples:**
```
x + y * z       # → x + (y * z)
2 ^ 3 ^ 2       # → 2 ^ (3 ^ 2) = 512  (right-associative)
a < b and b < c # → (a < b) and (b < c)
-x ^ 2          # → -(x ^ 2)  (unary has lower precedence than power)
```

## Scalar Functions (dew-scalar)

### Constants

| Function | Return | Description |
|----------|--------|-------------|
| `pi()` | Scalar | π ≈ 3.14159265359 |
| `e()` | Scalar | Euler's number ≈ 2.71828182846 |
| `tau()` | Scalar | τ = 2π ≈ 6.28318530718 |

### Trigonometry

| Function | Arguments | Return | Domain | Range | Description |
|----------|-----------|--------|--------|-------|-------------|
| `sin(x)` | Scalar | Scalar | ℝ | [-1, 1] | Sine |
| `cos(x)` | Scalar | Scalar | ℝ | [-1, 1] | Cosine |
| `tan(x)` | Scalar | Scalar | ℝ \ {π/2 + nπ} | ℝ | Tangent |
| `asin(x)` | Scalar | Scalar | [-1, 1] | [-π/2, π/2] | Arc sine |
| `acos(x)` | Scalar | Scalar | [-1, 1] | [0, π] | Arc cosine |
| `atan(x)` | Scalar | Scalar | ℝ | (-π/2, π/2) | Arc tangent |
| `atan2(y, x)` | 2× Scalar | Scalar | ℝ² | (-π, π] | Two-argument arc tangent |
| `sinh(x)` | Scalar | Scalar | ℝ | ℝ | Hyperbolic sine |
| `cosh(x)` | Scalar | Scalar | ℝ | [1, ∞) | Hyperbolic cosine |
| `tanh(x)` | Scalar | Scalar | ℝ | (-1, 1) | Hyperbolic tangent |

### Exponential & Logarithmic

| Function | Arguments | Return | Domain | Range | Description |
|----------|-----------|--------|--------|-------|-------------|
| `exp(x)` | Scalar | Scalar | ℝ | (0, ∞) | e^x |
| `exp2(x)` | Scalar | Scalar | ℝ | (0, ∞) | 2^x |
| `ln(x)` | Scalar | Scalar | (0, ∞) | ℝ | Natural logarithm |
| `log(x)` | Scalar | Scalar | (0, ∞) | ℝ | Natural logarithm (alias) |
| `log2(x)` | Scalar | Scalar | (0, ∞) | ℝ | Base-2 logarithm |
| `log10(x)` | Scalar | Scalar | (0, ∞) | ℝ | Base-10 logarithm |
| `pow(x, y)` | 2× Scalar | Scalar | x > 0 or y ∈ ℤ | ℝ | x^y |
| `sqrt(x)` | Scalar | Scalar | [0, ∞) | [0, ∞) | Square root |
| `inversesqrt(x)` | Scalar | Scalar | (0, ∞) | (0, ∞) | 1 / sqrt(x) |

### Common Math

| Function | Arguments | Return | Description |
|----------|-----------|--------|-------------|
| `abs(x)` | Scalar | Scalar | Absolute value \|x\| |
| `sign(x)` | Scalar | Scalar | Sign: -1 (x < 0), 0 (x = 0), 1 (x > 0) |
| `floor(x)` | Scalar | Scalar | Round down to nearest integer |
| `ceil(x)` | Scalar | Scalar | Round up to nearest integer |
| `round(x)` | Scalar | Scalar | Round to nearest integer |
| `trunc(x)` | Scalar | Scalar | Truncate toward zero |
| `fract(x)` | Scalar | Scalar | Fractional part: x - floor(x) |
| `min(a, b)` | 2× Scalar | Scalar | Minimum of a and b |
| `max(a, b)` | 2× Scalar | Scalar | Maximum of a and b |
| `clamp(x, lo, hi)` | 3× Scalar | Scalar | Constrain x to [lo, hi] |
| `saturate(x)` | Scalar | Scalar | Clamp to [0, 1] |

### Interpolation

| Function | Arguments | Return | Description |
|----------|-----------|--------|-------------|
| `lerp(a, b, t)` | 3× Scalar | Scalar | Linear interpolation: a + (b - a) * t |
| `mix(a, b, t)` | 3× Scalar | Scalar | Alias for lerp (GLSL naming) |
| `step(edge, x)` | 2× Scalar | Scalar | 0 if x < edge, else 1 |
| `smoothstep(e0, e1, x)` | 3× Scalar | Scalar | Smooth Hermite interpolation [0, 1] |
| `inverse_lerp(a, b, v)` | 3× Scalar | Scalar | Inverse lerp: (v - a) / (b - a) |
| `remap(x, in_lo, in_hi, out_lo, out_hi)` | 5× Scalar | Scalar | Remap x from [in_lo, in_hi] to [out_lo, out_hi] |

### Integer Operations

Available with `scalar_registry_int()` for i32/i64 types:

| Operator | Description |
|----------|-------------|
| `%` | Modulo |
| `&` | Bitwise AND |
| `\|` | Bitwise OR |
| `<<` | Left shift |
| `>>` | Right shift |
| `~x` | Bitwise NOT |

## Vector Functions (dew-linalg)

### Constructors

| Function | Arguments | Return | Description |
|----------|-----------|--------|-------------|
| `vec2(x, y)` | 2× Scalar | Vec2 | Construct 2D vector |
| `vec3(x, y, z)` | 3× Scalar | Vec3 | Construct 3D vector |
| `vec4(x, y, z, w)` | 4× Scalar | Vec4 | Construct 4D vector |

### Component Extraction

| Function | Argument | Return | Description |
|----------|----------|--------|-------------|
| `x(v)` | Vec2/3/4 | Scalar | Extract x component (index 0) |
| `y(v)` | Vec2/3/4 | Scalar | Extract y component (index 1) |
| `z(v)` | Vec3/4 | Scalar | Extract z component (index 2) |
| `w(v)` | Vec4 | Scalar | Extract w component (index 3) |

### Vector Operations

| Function | Arguments | Return | Description |
|----------|-----------|--------|-------------|
| `dot(a, b)` | 2× VecN | Scalar | Dot product: Σ(aᵢ × bᵢ) |
| `cross(a, b)` | 2× Vec3 | Vec3 | Cross product (3D only) |
| `length(v)` | VecN | Scalar | Vector magnitude: sqrt(dot(v, v)) |
| `distance(a, b)` | 2× VecN | Scalar | Distance: length(b - a) |
| `normalize(v)` | VecN | VecN | Unit vector: v / length(v) |
| `reflect(i, n)` | 2× VecN | VecN | Reflect i across normal n |
| `hadamard(a, b)` | 2× VecN | VecN | Element-wise multiply: [a₀×b₀, a₁×b₁, ...] |
| `lerp(a, b, t)` | 2× VecN, Scalar | VecN | Linear interpolation |
| `mix(a, b, t)` | 2× VecN, Scalar | VecN | Alias for lerp |

### Vectorized Math

These functions apply element-wise to vectors:

| Function | Argument | Return | Description |
|----------|----------|--------|-------------|
| `sin(v)` | VecN | VecN | Apply sin to each component |
| `cos(v)` | VecN | VecN | Apply cos to each component |
| `abs(v)` | VecN | VecN | Apply abs to each component |
| `floor(v)` | VecN | VecN | Apply floor to each component |
| `fract(v)` | VecN | VecN | Apply fract to each component |
| `sqrt(v)` | VecN | VecN | Apply sqrt to each component |
| `min(a, b)` | 2× VecN | VecN | Component-wise minimum |
| `max(a, b)` | 2× VecN | VecN | Component-wise maximum |
| `clamp(v, lo, hi)` | 3× VecN | VecN | Component-wise clamp |
| `step(edge, v)` | Scalar/VecN, VecN | VecN | Component-wise step |
| `smoothstep(e0, e1, v)` | Scalar, Scalar, VecN | VecN | Component-wise smoothstep |

### Rotation (2D)

| Function | Arguments | Return | Description |
|----------|-----------|--------|-------------|
| `rotate2d(v, angle)` | Vec2, Scalar | Vec2 | Rotate 2D vector by angle (radians) |

### Rotation (3D)

| Function | Arguments | Return | Description |
|----------|-----------|--------|-------------|
| `rotate_x(v, angle)` | Vec3, Scalar | Vec3 | Rotate around X axis |
| `rotate_y(v, angle)` | Vec3, Scalar | Vec3 | Rotate around Y axis |
| `rotate_z(v, angle)` | Vec3, Scalar | Vec3 | Rotate around Z axis |
| `rotate3d(v, axis, angle)` | Vec3, Vec3, Scalar | Vec3 | Rotate around arbitrary axis (Rodrigues' formula) |

### Matrix Constructors

| Function | Arguments | Return | Description |
|----------|-----------|--------|-------------|
| `mat2(...)` | 4× Scalar | Mat2 | 2×2 matrix (column-major) |
| `mat3(...)` | 9× Scalar | Mat3 | 3×3 matrix (column-major) |
| `mat4(...)` | 16× Scalar | Mat4 | 4×4 matrix (column-major) |

### Matrix Operations

| Expression | Types | Return | Description |
|------------|-------|--------|-------------|
| `m * v` | MatN × VecN | VecN | Matrix-vector multiply (column vector) |
| `v * m` | VecN × MatN | VecN | Vector-matrix multiply (row vector) |
| `m1 * m2` | MatN × MatN | MatN | Matrix multiplication |
| `m * s` | MatN × Scalar | MatN | Scale matrix |

## Complex Functions (dew-complex)

### Component Access

| Function | Argument | Return | Description |
|----------|----------|--------|-------------|
| `re(z)` | Complex | Scalar | Real part |
| `im(z)` | Complex | Scalar | Imaginary part |

### Properties

| Function | Argument | Return | Description |
|----------|----------|--------|-------------|
| `abs(z)` | Complex | Scalar | Magnitude \|z\| = √(re² + im²) |
| `arg(z)` | Complex | Scalar | Phase angle: atan2(im, re) |
| `norm(z)` | Complex | Scalar | Squared magnitude: re² + im² |
| `conj(z)` | Complex | Complex | Conjugate: re - i×im |

### Exponential & Logarithmic

| Function | Argument | Return | Description |
|----------|----------|--------|-------------|
| `exp(z)` | Complex | Complex | e^z = e^re × (cos(im) + i×sin(im)) |
| `log(z)` | Complex | Complex | ln(z) = ln\|z\| + i×arg(z) |
| `sqrt(z)` | Complex | Complex | Principal square root |
| `pow(z, n)` | Complex, Scalar | Complex | z^n |

### Construction

| Function | Arguments | Return | Description |
|----------|-----------|--------|-------------|
| `polar(r, theta)` | 2× Scalar | Complex | From polar: r × e^(i×theta) |

### Operators

| Expression | Types | Return | Description |
|------------|-------|--------|-------------|
| `z1 + z2` | Complex + Complex | Complex | Addition |
| `z1 - z2` | Complex - Complex | Complex | Subtraction |
| `z1 * z2` | Complex × Complex | Complex | Multiplication |
| `z1 / z2` | Complex / Complex | Complex | Division |
| `z ^ n` | Complex ^ Scalar | Complex | Power |
| `-z` | -Complex | Complex | Negation |
| `s * z` | Scalar × Complex | Complex | Scale |

## Quaternion Functions (dew-quaternion)

Quaternions use **[x, y, z, w]** order (scalar-last convention).

### Properties

| Function | Argument | Return | Description |
|----------|----------|--------|-------------|
| `length(q)` | Quaternion | Scalar | Magnitude: √(x² + y² + z² + w²) |
| `normalize(q)` | Quaternion | Quaternion | Unit quaternion (for rotations) |
| `dot(q1, q2)` | 2× Quaternion | Scalar | Dot product |
| `conj(q)` | Quaternion | Quaternion | Conjugate: [-x, -y, -z, w] |
| `inverse(q)` | Quaternion | Quaternion | Multiplicative inverse: conj(q) / norm(q) |

### Construction

| Function | Arguments | Return | Description |
|----------|-----------|--------|-------------|
| `axis_angle(axis, angle)` | Vec3, Scalar | Quaternion | From axis-angle representation |

### Rotation

| Expression | Types | Return | Description |
|------------|-------|--------|-------------|
| `q * v` | Quaternion × Vec3 | Vec3 | Rotate vector by quaternion |
| `q1 * q2` | Quaternion × Quaternion | Quaternion | Combine rotations (q1 then q2) |

### Interpolation

| Function | Arguments | Return | Description |
|----------|-----------|--------|-------------|
| `slerp(q1, q2, t)` | 2× Quaternion, Scalar | Quaternion | Spherical linear interpolation |
| `lerp(q1, q2, t)` | 2× Quaternion, Scalar | Quaternion | Linear interpolation |

### Vector Operations

| Function | Arguments | Return | Description |
|----------|-----------|--------|-------------|
| `cross(a, b)` | 2× Vec3 | Vec3 | Cross product |
| `length(v)` | Vec3 | Scalar | Vector magnitude |
| `normalize(v)` | Vec3 | Vec3 | Unit vector |

## Conditional Expressions (feature = "cond")

### Comparison Operators

| Operator | Description | Result |
|----------|-------------|--------|
| `<` | Less than | 0 or 1 |
| `<=` | Less than or equal | 0 or 1 |
| `>` | Greater than | 0 or 1 |
| `>=` | Greater than or equal | 0 or 1 |
| `==` | Equal | 0 or 1 |
| `!=` | Not equal | 0 or 1 |

Boolean results are represented as scalars: 0.0 = false, 1.0 = true.

### Logical Operators

| Operator | Description | Evaluation |
|----------|-------------|------------|
| `and` | Logical AND | Short-circuit: if left is 0, returns 0 without evaluating right |
| `or` | Logical OR | Short-circuit: if left is 1, returns 1 without evaluating right |
| `not` | Logical NOT | Unary: returns 1 if operand is 0, else 0 |

### If-Then-Else

```
if condition then value_if_true else value_if_false
```

**Examples:**
```
if x > 0 then sqrt(x) else 0
if a < b and b < c then 1 else 0
if x == 0 then y else x / y
```

## Type Compatibility

### Scalar Operations

| Left | Operator | Right | Result |
|------|----------|-------|--------|
| Scalar | `+`, `-`, `*`, `/`, `^` | Scalar | Scalar |

### Vector-Scalar Operations

| Left | Operator | Right | Result |
|------|----------|-------|--------|
| VecN | `+`, `-` | VecN | VecN |
| VecN | `*`, `/` | Scalar | VecN |
| Scalar | `*` | VecN | VecN |

### Matrix Operations

| Left | Operator | Right | Result |
|------|----------|-------|--------|
| MatN | `*` | VecN | VecN (column vector) |
| VecN | `*` | MatN | VecN (row vector) |
| MatN | `*` | MatN | MatN |
| MatN | `*`, `/` | Scalar | MatN |

### Complex Operations

| Left | Operator | Right | Result |
|------|----------|-------|--------|
| Complex | `+`, `-`, `*`, `/` | Complex | Complex |
| Complex | `*`, `/` | Scalar | Complex |
| Scalar | `*` | Complex | Complex |
| Complex | `^` | Scalar | Complex |

### Quaternion Operations

| Left | Operator | Right | Result |
|------|----------|-------|--------|
| Quaternion | `+`, `-` | Quaternion | Quaternion |
| Quaternion | `*` | Quaternion | Quaternion (Hamilton product) |
| Quaternion | `*` | Vec3 | Vec3 (rotation) |
| Quaternion | `*`, `/` | Scalar | Quaternion |
| Scalar | `*` | Quaternion | Quaternion |

## Let Bindings

Syntax:
```
let name = value; body
```

**Scope:** The bound variable is only visible in the body expression.

**Chaining:**
```
let a = x * 2; let b = a + 1; b * b
```

**Shadowing:** Inner bindings hide outer ones with the same name.

**Backend Support:**
- **Lua/Cranelift:** Native support
- **WGSL/GLSL:** Requires `LetInlining` pass (included in `standard_passes()`)

## Error Handling

### Parse Errors

```rust
Expr::parse("x + + y")  // Error: unexpected token at position 4
```

### Evaluation Errors

```rust
eval(ast, &vars, &registry)
// UnknownVariable("x") - variable not in vars map
// UnknownFunction("foo") - function not registered
// TypeMismatch - incompatible operand types
// DivideByZero - division by zero (if checked)
```

### Type Errors

Domain crates enforce type safety during evaluation and codegen:
```rust
// Error: can't add Vec2 and Vec3
emit_wgsl("vec2(1, 2) + vec3(1, 2, 3)", &var_types)
```
