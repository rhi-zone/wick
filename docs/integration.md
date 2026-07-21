# Integration Guide

How to integrate Dew into your project.

## Game Engines

### Bevy

Integrate Dew for hot-reloadable game logic or GPU compute shaders.

**Setup:**
```toml
[dependencies]
bevy = "0.12"
dew-core = { version = "0.1", features = ["cond", "func"] }
dew-scalar = { version = "0.1", features = ["lua", "wgsl"] }
dew-linalg = { version = "0.1", features = ["3d", "wgsl"] }
```

**Hot-Reloadable Damage Formula:**
```rust
use bevy::prelude::*;
use dew_core::Expr;
use dew_scalar::{eval, scalar_registry};

#[derive(Resource)]
struct GameFormulas {
    damage: Expr,
    registry: FunctionRegistry<f32, f32>,
}

fn setup(mut commands: Commands) {
    let damage = Expr::parse(r#"
        let base = attack * 1.5;
        let crit = if rand() > 0.75 then base * 2 else base;
        max(1, floor(crit * (1 - defense / 100)))
    "#).unwrap();

    commands.insert_resource(GameFormulas {
        damage,
        registry: scalar_registry(),
    });
}

fn calculate_damage(
    formulas: Res<GameFormulas>,
    attacker: &Character,
    defender: &Character,
) -> i32 {
    let vars = [
        ("attack".to_string(), attacker.attack as f32),
        ("defense".to_string(), defender.defense as f32),
    ].into();

    eval(formulas.damage.ast(), &vars, &formulas.registry)
        .unwrap() as i32
}
```

**WGSL Compute Shader:**
```rust
use bevy::render::render_resource::*;
use dew_linalg::{emit_wgsl, Type};

fn create_particle_shader(device: &RenderDevice) -> ShaderModule {
    // Define particle update logic
    let expr = Expr::parse(r#"
        let gravity = vec3(0, -9.8, 0);
        let drag = velocity * -0.1;
        position + velocity * dt + 0.5 * (gravity + drag) * dt * dt
    "#).unwrap();

    let var_types = [
        ("position".to_string(), Type::Vec3),
        ("velocity".to_string(), Type::Vec3),
        ("dt".to_string(), Type::Scalar),
    ].into();

    let wgsl = emit_wgsl(expr.ast(), &var_types).unwrap();

    let shader_source = format!(r#"
        struct Particle {{
            position: vec3<f32>,
            velocity: vec3<f32>,
        }}

        @group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
        @group(0) @binding(1) var<uniform> dt: f32;

        @compute @workgroup_size(64)
        fn update(@builtin(global_invocation_id) id: vec3<u32>) {{
            let index = id.x;
            let p = particles[index];
            particles[index].position = {};
        }}
    "#, wgsl.code);

    device.create_shader_module(ShaderModuleDescriptor {
        label: Some("particle_update"),
        source: ShaderSource::Wgsl(shader_source.into()),
    })
}
```

### Custom Engine

**Resource Loading:**
```rust
use dew_core::Expr;
use std::collections::HashMap;

struct ExpressionAsset {
    source: String,
    ast: Ast,
    optimized: Ast,
}

struct ExpressionLoader;

impl AssetLoader for ExpressionLoader {
    fn load(&self, path: &Path) -> Result<ExpressionAsset, Error> {
        let source = std::fs::read_to_string(path)?;
        let expr = Expr::parse(&source)?;
        let optimized = optimize(expr.ast().clone(), &standard_passes());

        Ok(ExpressionAsset {
            source,
            ast: expr.into_ast(),
            optimized,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["dew"]
    }
}
```

## Web Integration

### WASM + JavaScript

**Rust (WASM):**
```rust
use wasm_bindgen::prelude::*;
use dew_scalar::{eval, scalar_registry};

#[wasm_bindgen]
pub struct DewEngine {
    registry: FunctionRegistry<f32, f32>,
}

#[wasm_bindgen]
impl DewEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            registry: scalar_registry(),
        }
    }

    #[wasm_bindgen]
    pub fn eval(&self, source: &str, vars: JsValue) -> Result<f32, JsValue> {
        let expr = Expr::parse(source)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let vars: HashMap<String, f32> = serde_wasm_bindgen::from_value(vars)?;

        eval(expr.ast(), &vars, &self.registry)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
```

**JavaScript:**
```javascript
import init, { DewEngine } from './dew_wasm.js';

await init();
const engine = new DewEngine();

// Evaluate expression
const result = engine.eval('sin(x) + cos(y)', { x: 1.0, y: 0.5 });
console.log(result);

// Use in animation loop
function animate(time) {
    const value = engine.eval(
        'sin(time * 0.001) * 100',
        { time }
    );
    element.style.transform = `translateX(${value}px)`;
    requestAnimationFrame(animate);
}
```

### Three.js / WebGPU

```javascript
// Generate WGSL shader from Dew expression
const shaderCode = engine.emitWGSL(
    'normalize(cross(a, b))',
    { a: 'Vec3', b: 'Vec3' }
);

// Use in WebGPU compute shader
const shaderModule = device.createShaderModule({
    code: `
        @group(0) @binding(0) var<storage, read_write> output: array<vec3<f32>>;

        @compute @workgroup_size(64)
        fn main(@builtin(global_invocation_id) id: vec3<u32>) {
            let a = vec3<f32>(1.0, 0.0, 0.0);
            let b = vec3<f32>(0.0, 1.0, 0.0);
            output[id.x] = ${shaderCode};
        }
    `
});
```

## Graphics APIs

### wgpu

**Shader Generation Pipeline:**
```rust
use dew_linalg::{emit_wgsl, Type};
use wgpu::*;

struct ComputePipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: BindGroupLayout,
}

impl ComputePipeline {
    fn from_expr(device: &Device, expr: &Ast, var_types: &HashMap<String, Type>) -> Self {
        let wgsl = emit_wgsl(expr, var_types).unwrap();

        // Wrap in compute shader template
        let shader_code = format!(r#"
            @group(0) @binding(0) var<storage, read> input: array<f32>;
            @group(0) @binding(1) var<storage, read_write> output: array<f32>;

            @compute @workgroup_size(256)
            fn main(@builtin(global_invocation_id) id: vec3<u32>) {{
                let idx = id.x;
                let value = input[idx];
                output[idx] = {};
            }}
        "#, wgsl.code);

        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("dew_compute"),
            source: ShaderSource::Wgsl(shader_code.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("dew_compute_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("dew_compute_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("dew_compute"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }
}
```

### OpenGL (via GLSL)

```rust
use dew_linalg::emit_glsl;

fn create_gl_shader(expr: &Ast, var_types: &HashMap<String, Type>) -> String {
    let glsl = emit_glsl(expr, var_types).unwrap();

    format!(r#"
        #version 450 core

        layout(local_size_x = 256) in;

        layout(std430, binding = 0) buffer Input {{
            vec3 input_data[];
        }};

        layout(std430, binding = 1) buffer Output {{
            vec3 output_data[];
        }};

        void main() {{
            uint idx = gl_GlobalInvocationID.x;
            vec3 value = input_data[idx];
            output_data[idx] = {};
        }}
    "#, glsl.code)
}
```

### C Code Generation

The C backend generates code for embedding in C/C++ projects with custom math libraries.

```rust
use dew_linalg::{emit_c, emit_c_fn, Type};

// Generate inline expression
let expr = Expr::parse("dot(a, b) + length(c)").unwrap();
let var_types = [
    ("a".to_string(), Type::Vec3),
    ("b".to_string(), Type::Vec3),
    ("c".to_string(), Type::Vec3),
].into();

let c_expr = emit_c(expr.ast(), &var_types).unwrap();
// c_expr.code = "(vec3_dot(a, b) + vec3_length(c))"

// Generate complete function
let func = emit_c_fn(
    "compute_value",
    expr.ast(),
    &[("a", Type::Vec3), ("b", Type::Vec3), ("c", Type::Vec3)],
    Type::Scalar,
).unwrap();
// float compute_value(vec3 a, vec3 b, vec3 c) {
//     return (vec3_dot(a, b) + vec3_length(c));
// }
```

**Example header for generated code:**
```c
// mymath.h - User provides these definitions
typedef struct { float x, y, z; } vec3;

vec3 vec3_add(vec3 a, vec3 b);
vec3 vec3_sub(vec3 a, vec3 b);
vec3 vec3_scale(vec3 v, float s);
float vec3_dot(vec3 a, vec3 b);
float vec3_length(vec3 v);
vec3 vec3_normalize(vec3 v);
vec3 vec3_cross(vec3 a, vec3 b);

// Generated Dew code uses these functions
#include "generated_expressions.c"
```

## Audio Processing

### CPAL Integration

```rust
use cpal::traits::*;
use dew_complex::{eval_lua, Value, complex_registry};

struct AudioProcessor {
    expr: Expr,
    registry: FunctionRegistry<f32, Value<f32>>,
    sample_rate: f32,
    phase: f32,
}

impl AudioProcessor {
    fn process(&mut self, output: &mut [f32]) {
        let vars = [
            ("sample_rate".to_string(), Value::Scalar(self.sample_rate)),
        ];

        for sample in output.iter_mut() {
            let mut vars = vars.clone();
            vars.insert("phase".to_string(), Value::Scalar(self.phase));

            *sample = match eval_lua(self.expr.ast(), &vars.into()).unwrap() {
                Value::Scalar(s) => s,
                _ => 0.0,
            };

            self.phase += 1.0;
        }
    }
}

// Create audio stream
let device = host.default_output_device().unwrap();
let config = device.default_output_config().unwrap();

let mut processor = AudioProcessor {
    expr: Expr::parse("sin(phase * 440 * 2 * pi() / sample_rate)").unwrap(),
    registry: complex_registry(),
    sample_rate: config.sample_rate().0 as f32,
    phase: 0.0,
};

let stream = device.build_output_stream(
    &config.into(),
    move |data: &mut [f32], _| processor.process(data),
    |err| eprintln!("Stream error: {}", err),
    None,
).unwrap();
```

## Embedded Systems

### no_std Support

```toml
[dependencies]
dew-core = { version = "0.1", default-features = false, features = ["func"] }
dew-scalar = { version = "0.1", default-features = false }
```

**Microcontroller Example:**
```rust
#![no_std]
#![no_main]

use dew_scalar::{eval, scalar_registry_int};

#[entry]
fn main() -> ! {
    // Pre-parsed at compile time
    static EXPR: OnceCell<Expr> = OnceCell::new();
    EXPR.set(Expr::parse("(sensor_value * 100) / 1023").unwrap());

    let registry = scalar_registry_int();

    loop {
        let sensor_value = read_adc() as i32;
        let vars = [("sensor_value".to_string(), sensor_value)].into();

        let percentage = eval::<i32>(EXPR.get().unwrap().ast(), &vars, &registry)
            .unwrap();

        display_value(percentage);
        delay_ms(100);
    }
}
```

### Lua VM Integration

For systems with a Lua interpreter:

```rust
use dew_scalar::emit_lua_code;

// Generate Lua code once
let lua_code = emit_lua_code(expr.ast(), &var_types).unwrap();

// Write to file for embedded Lua VM
std::fs::write("sensor_logic.lua", format!(r#"
    function process(sensor_value)
        return {}
    end
"#, lua_code.code)).unwrap();

// Embedded system loads and executes Lua
// Can be updated without reflashing firmware
```

## Testing & Validation

### Property-Based Testing

```rust
use proptest::prelude::*;
use dew_linalg::{eval, emit_wgsl, Type};

proptest! {
    #[test]
    fn test_expr_consistency(
        x in -10.0..10.0_f32,
        y in -10.0..10.0_f32,
    ) {
        let expr = Expr::parse("x * 2 + y").unwrap();

        // Eval should match manual calculation
        let vars = [
            ("x".to_string(), Value::Scalar(x)),
            ("y".to_string(), Value::Scalar(y)),
        ].into();

        let result = eval(expr.ast(), &vars, &linalg_registry()).unwrap();
        let expected = Value::Scalar(x * 2.0 + y);

        match (result, expected) {
            (Value::Scalar(r), Value::Scalar(e)) => {
                assert!((r - e).abs() < 1e-6);
            }
            _ => panic!("Type mismatch"),
        }
    }
}
```

### Backend Parity Tests

```rust
#[cfg(all(feature = "lua", feature = "cranelift"))]
#[test]
fn test_backend_parity() {
    let expr = Expr::parse("sqrt(x*x + y*y)").unwrap();

    let vars = [
        ("x".to_string(), 3.0),
        ("y".to_string(), 4.0),
    ].into();

    // Reference: direct eval
    let eval_result = eval(expr.ast(), &vars, &scalar_registry()).unwrap();

    // Lua backend
    let lua_result = eval_lua(expr.ast(), &vars).unwrap();

    // Cranelift JIT
    let jit_fn = compile_cranelift(expr.ast(), &["x", "y"]).unwrap();
    let jit_result = jit_fn(&[3.0, 4.0]);

    assert!((eval_result - 5.0).abs() < 1e-6);
    assert!((lua_result - 5.0).abs() < 1e-6);
    assert!((jit_result - 5.0).abs() < 1e-6);
}
```

## Configuration Patterns

### Expression Files

**config/formulas.dew:**
```
# Damage calculation
damage = (attack * 1.5) * (1 - defense / 100)

# Experience curve
exp_for_level = floor(100 * (level ^ 1.5))

# Movement speed
speed = base_speed * (1 + agility / 100)
```

**Loading:**
```rust
use std::collections::HashMap;

struct FormulaConfig {
    formulas: HashMap<String, Expr>,
}

impl FormulaConfig {
    fn load(path: &Path) -> Result<Self, Error> {
        let content = std::fs::read_to_string(path)?;
        let mut formulas = HashMap::new();

        for line in content.lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            let parts: Vec<_> = line.split('=').collect();
            if parts.len() == 2 {
                let name = parts[0].trim().to_string();
                let expr = Expr::parse(parts[1].trim())?;
                formulas.insert(name, expr);
            }
        }

        Ok(Self { formulas })
    }

    fn get(&self, name: &str) -> Option<&Expr> {
        self.formulas.get(name)
    }
}
```

### Environment-Specific Optimization

```rust
struct OptimizedExpressions {
    dev: Ast,    // No optimization for debugging
    prod: Ast,   // Full optimization
}

impl OptimizedExpressions {
    fn new(source: &str) -> Result<Self, ParseError> {
        let expr = Expr::parse(source)?;

        Ok(Self {
            dev: expr.ast().clone(),
            prod: optimize(expr.ast().clone(), &standard_passes()),
        })
    }

    fn get(&self) -> &Ast {
        if cfg!(debug_assertions) {
            &self.dev
        } else {
            &self.prod
        }
    }
}
```

## Plugin Systems

### User-Defined Functions

```rust
use dew_core::{ExprFn, FunctionRegistry};

// Allow users to register custom functions
pub struct PluginRegistry<T> {
    registry: FunctionRegistry<T, Value<T>>,
}

impl<T> PluginRegistry<T> {
    pub fn register_plugin<F>(&mut self, func: F)
    where
        F: ExprFn<T, Value<T>> + 'static,
    {
        self.registry.register(func);
    }
}

// User plugin
struct CustomNoise;

impl ExprFn<f32, Value<f32>> for CustomNoise {
    fn name(&self) -> &str { "custom_noise" }
    fn arg_count(&self) -> usize { 2 }

    fn call(&self, args: &[Value<f32>]) -> Value<f32> {
        // Custom implementation
        todo!()
    }
}

// User registers their function
let mut registry = PluginRegistry::new();
registry.register_plugin(CustomNoise);
```

## Summary

Key integration patterns:
- **Game engines**: Hot-reloadable logic, GPU compute shaders
- **Web**: WASM + JavaScript, WebGPU shaders
- **Graphics**: wgpu, OpenGL via WGSL/GLSL generation
- **Audio**: Real-time DSP with complex numbers
- **Embedded**: no_std support, Lua code generation
- **Testing**: Backend parity tests, property-based validation
- **Configuration**: Expression files, environment-specific optimization
- **Plugins**: User-defined function registration
