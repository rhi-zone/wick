# Use Cases

Real-world applications and patterns for Dew expressions.

## GPU Shaders

### Procedural Generation

Generate complex procedural patterns once in Dew, compile to WGSL for GPU execution.

**Before (manual WGSL):**
```wgsl
fn noise_octaves(p: vec2<f32>, octaves: i32) -> f32 {
    var value = 0.0;
    var amplitude = 1.0;
    var frequency = 1.0;
    var max_value = 0.0;

    for (var i = 0; i < octaves; i++) {
        value += noise(p * frequency) * amplitude;
        max_value += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    return value / max_value;
}
```

**After (Dew):**
```rust
use dew_linalg::{emit_wgsl, Type};

// Define expression once
let expr = Expr::parse(r#"
    let freq = 1.0;
    let amp = 1.0;
    let octave1 = noise(p * freq) * amp;
    let octave2 = noise(p * (freq * 2)) * (amp * 0.5);
    let octave3 = noise(p * (freq * 4)) * (amp * 0.25);
    (octave1 + octave2 + octave3) / 1.75
"#).unwrap();

// Generate WGSL
let var_types = [("p".to_string(), Type::Vec2)].into();
let wgsl = emit_wgsl(expr.ast(), &var_types).unwrap();

// Use in shader pipeline
let shader = format!(r#"
    fn procedural_noise(p: vec2<f32>) -> f32 {{
        return {};
    }}
"#, wgsl.code);
```

**Benefits:**
- Single source of truth for the algorithm
- Can test with Cranelift/Lua before deploying to GPU
- Easy to tweak parameters and regenerate
- Expression optimization automatically applied

### Particle Systems

```rust
// Particle velocity update
let expr = Expr::parse(r#"
    let gravity = vec3(0, -9.8, 0);
    let drag = vel * -0.1;
    let force = gravity + drag;
    vel + force * dt
"#).unwrap();

// Compile to WGSL compute shader
let wgsl = emit_wgsl(expr.ast(), &var_types).unwrap();
```

### Color Grading

```rust
// Complex color transformation
let expr = Expr::parse(r#"
    let adjusted = (color - 0.5) * contrast + 0.5;
    let saturated = mix(vec3(luminance), adjusted, saturation);
    clamp(saturated + brightness, 0.0, 1.0)
"#).unwrap();
```

## Signal Processing

### Audio Effects with Complex Numbers

Complex numbers in Dew are perfect for frequency-domain audio processing.

```rust
use dew_complex::{Value, eval, complex_registry};

// Frequency response of a low-pass filter
let expr = Expr::parse(r#"
    let omega = freq * 2 * pi() / sample_rate;
    let z = polar(1.0, omega);
    let H = 1.0 / (1.0 + z * (-cutoff));
    abs(H)
"#).unwrap();

// Evaluate at different frequencies
let registry = complex_registry();
for freq in [100.0, 500.0, 1000.0, 5000.0] {
    let vars = [
        ("freq".to_string(), Value::Scalar(freq)),
        ("sample_rate".to_string(), Value::Scalar(44100.0)),
        ("cutoff".to_string(), Value::Complex([0.7, 0.3])),
    ].into();

    let response = eval(expr.ast(), &vars, &registry).unwrap();
    println!("{}Hz: {:?}", freq, response);
}
```

### Phase Vocoder

```rust
// Phase adjustment for pitch shifting
let expr = Expr::parse(r#"
    let phase_diff = arg(current) - arg(previous);
    let unwrapped = phase_diff + 2*pi() * round(phase_diff / (-2*pi()));
    let shifted = previous * polar(1.0, unwrapped * pitch_ratio);
    shifted
"#).unwrap();
```

## 3D Graphics & Animation

### Quaternion Interpolation

Smooth camera transitions or skeletal animation.

```rust
use dew_quaternion::{Value, eval, quaternion_registry};

// Camera interpolation
let expr = Expr::parse(r#"
    let interpolated_rot = slerp(start_rot, end_rot, t);
    let interpolated_pos = lerp(start_pos, end_pos, smoothstep(0, 1, t));
    rotate(camera_forward, interpolated_rot)
"#).unwrap();

// Evaluate for each frame
let registry = quaternion_registry();
for frame in 0..60 {
    let t = frame as f32 / 60.0;
    let vars = [
        ("start_rot".to_string(), Value::Quaternion([0.0, 0.0, 0.0, 1.0])),
        ("end_rot".to_string(), Value::Quaternion([0.0, 0.707, 0.0, 0.707])),
        ("start_pos".to_string(), Value::Vec3([0.0, 0.0, 0.0])),
        ("end_pos".to_string(), Value::Vec3([10.0, 5.0, 0.0])),
        ("camera_forward".to_string(), Value::Vec3([0.0, 0.0, -1.0])),
        ("t".to_string(), Value::Scalar(t)),
    ].into();

    let result = eval(expr.ast(), &vars, &registry).unwrap();
    // Update camera transform
}
```

### Inverse Kinematics Helper

```rust
// Two-bone IK (shoulder-elbow-wrist)
let expr = Expr::parse(r#"
    let to_target = normalize(target - shoulder);
    let distance = length(target - shoulder);
    let clamped_dist = clamp(distance, 0, upper_len + lower_len);

    let cos_angle = (upper_len*upper_len + lower_len*lower_len - clamped_dist*clamped_dist)
                    / (2 * upper_len * lower_len);
    let elbow_angle = acos(clamp(cos_angle, -1, 1));

    axis_angle(cross(to_target, up), elbow_angle)
"#).unwrap();
```

## Game Development

### Hot-Reloadable Logic with Lua Backend

```rust
use dew_scalar::{emit_lua, eval_lua, scalar_registry};

// Define game logic expression
let damage_formula = Expr::parse(r#"
    let base_dmg = attack * 2;
    let crit_dmg = if rand > crit_chance then base_dmg * 2 else base_dmg;
    let final_dmg = crit_dmg * (1 - defense / 100);
    max(1, floor(final_dmg))
"#).unwrap();

// Hot-reload: just re-parse the expression from a file
// No need to recompile the game

// Evaluate with Lua (or Cranelift for JIT)
let vars = [
    ("attack".to_string(), 50.0),
    ("defense".to_string(), 20.0),
    ("crit_chance".to_string(), 0.25),
    ("rand".to_string(), 0.8),
].into();

let damage = eval_lua(damage_formula.ast(), &vars).unwrap();
```

### Procedural World Generation

```rust
// Terrain height calculation
let expr = Expr::parse(r#"
    let continental = noise(pos * 0.001) * 1000;
    let mountains = noise(pos * 0.01) * 200 * smoothstep(0.3, 0.7, continental / 1000);
    let hills = noise(pos * 0.05) * 50;
    let detail = noise(pos * 0.2) * 5;
    continental + mountains + hills + detail
"#).unwrap();

// Compile to Cranelift for fast native execution
#[cfg(feature = "cranelift")]
let jit_fn = emit_cranelift(expr.ast(), &var_types).unwrap();

// Generate chunk of terrain
for x in 0..256 {
    for z in 0..256 {
        let height = jit_fn(&[x as f64, z as f64]);
        chunk.set_height(x, z, height);
    }
}
```

## Scientific Computing

### Physical Simulations

```rust
// Spring-mass damper system
let expr = Expr::parse(r#"
    let spring_force = -k * (pos - rest_len);
    let damping_force = -c * vel;
    let acceleration = (spring_force + damping_force) / mass;
    acceleration
"#).unwrap();

// Integrate with RK4 or similar
```

### Vector Field Visualization

```rust
// Electromagnetic field at a point
let expr = Expr::parse(r#"
    let r = pos - charge_pos;
    let dist = length(r);
    let E = (charge / (dist * dist * dist)) * r;
    E
"#).unwrap();

// Generate field for visualization
let field = emit_wgsl(expr.ast(), &var_types).unwrap();
// Use in compute shader to generate arrow field
```

## Machine Learning

### Custom Activation Functions

```rust
// Swish activation: x * sigmoid(beta * x)
let expr = Expr::parse(r#"
    let sigmoid = 1.0 / (1.0 + exp(-beta * x));
    x * sigmoid
"#).unwrap();

// Test on CPU with Cranelift, deploy to GPU with WGSL
```

### Loss Function Prototyping

```rust
// Custom smooth L1 loss
let expr = Expr::parse(r#"
    let diff = abs(pred - target);
    if diff < threshold
        then 0.5 * diff * diff / threshold
        else diff - 0.5 * threshold
"#).unwrap();
```

## Embedded Systems

### Sensor Fusion

```rust
// Complementary filter for IMU
let expr = Expr::parse(r#"
    let gyro_angle = prev_angle + gyro * dt;
    let accel_angle = atan2(accel_y, accel_x);
    let fused = alpha * gyro_angle + (1 - alpha) * accel_angle;
    fused
"#).unwrap();

// Compile to Lua for microcontroller with Lua VM
// Or evaluate directly in Rust
```

## Data Visualization

### Custom Interpolation Schemes

```rust
// Perceptually uniform color interpolation in LAB space
let expr = Expr::parse(r#"
    let lab1 = rgb_to_lab(color1);
    let lab2 = rgb_to_lab(color2);
    let mixed_lab = lerp(lab1, lab2, t);
    lab_to_rgb(mixed_lab)
"#).unwrap();
```

## Performance Patterns

### Multi-Backend Strategy

```rust
// 1. Prototype with eval (simplest)
let result = eval(expr.ast(), &vars, &registry).unwrap();

// 2. Speed up with Cranelift JIT
#[cfg(feature = "cranelift")]
let jit_fn = compile_cranelift(expr.ast()).unwrap();
let result = jit_fn(&inputs);

// 3. Deploy to GPU with WGSL
let wgsl = emit_wgsl(expr.ast(), &var_types).unwrap();
// Use in wgpu compute shader

// Same expression, optimized for each context
```

### Expression Caching

```rust
use dew_core::optimize::{optimize, standard_passes};
use std::collections::HashMap;

struct ExpressionCache {
    cache: HashMap<String, Ast>,
}

impl ExpressionCache {
    fn get_optimized(&mut self, source: &str) -> Result<&Ast, ParseError> {
        if !self.cache.contains_key(source) {
            let expr = Expr::parse(source)?;
            let optimized = optimize(expr.ast().clone(), &standard_passes());
            self.cache.insert(source.to_string(), optimized);
        }
        Ok(&self.cache[source])
    }
}
```

## Cross-Platform Deployment

One expression, multiple targets:

```rust
let expr = Expr::parse("normalize(cross(a, b))").unwrap();

// Desktop: Cranelift JIT
#[cfg(all(not(target_arch = "wasm32"), feature = "cranelift"))]
let result = eval_cranelift(expr.ast(), &vars).unwrap();

// Web: WASM interpreter
#[cfg(target_arch = "wasm32")]
let result = eval(expr.ast(), &vars, &registry).unwrap();

// GPU: WGSL shader
#[cfg(feature = "wgpu")]
let shader = emit_wgsl(expr.ast(), &var_types).unwrap();

// Embedded: Lua VM
#[cfg(feature = "lua")]
let result = eval_lua(expr.ast(), &vars).unwrap();
```

## Summary

Dew shines when you need:
- **Cross-platform math** - Write once, run on CPU/GPU/web
- **Hot-reloadable logic** - Update expressions without recompiling
- **Domain-specific expressions** - GPU shaders, signal processing, 3D transforms
- **Rapid prototyping** - Test on CPU, deploy to GPU
- **Embedded constraints** - Small footprint, optional backends
