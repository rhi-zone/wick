# dew-wasm

WebAssembly bindings for the Dew expression language.

## Overview

`dew-wasm` exposes Dew's parser and backend code generators to JavaScript
via `wasm-bindgen`, so a web application can parse a Dew expression and emit
WGSL, GLSL, or Lua source without a server round-trip. It's built as both a
`cdylib` (for `wasm-pack`) and an `rlib`, and organizes its bindings into
per-domain modules (`scalar`, `linalg`, `complex`, `quaternion`) that mirror
the Rust domain crates. This crate is `publish = false` — it's a wasm-pack
build target, not a crates.io package.

## Feature flags (module profiles)

Rather than one flag per backend, `dew-wasm` selects which domain(s) to
compile in via profile features, since WASM binary size matters:

| Feature | Pulls in | Description |
|---------|----------|-------------|
| `core` | `dew-scalar` | Scalar math only (default) |
| `linalg` | `core` + `dew-linalg` | Adds vectors/matrices |
| `graphics` | `linalg` + `dew-quaternion` | Adds quaternions for 3D graphics |
| `signal` | `core` + `dew-complex` | Adds complex numbers for signal processing |
| `full` | `dew-linalg`, `dew-complex`, `dew-quaternion` | Everything |
| `console_error_panic_hook` | — | Forwards Rust panics to the browser console (default) |

Each included domain crate is built with `wgsl`, `glsl`, and `lua-codegen`
backend features enabled.

## Building

```bash
# Build the default (core) profile
wasm-pack build --target web

# Build the full profile
wasm-pack build --target web --no-default-features --features full,console_error_panic_hook
```

## JavaScript API

### Parsing

```js
import init, { parse } from "./pkg/dew_wasm.js";

await init();

const result = parse("sin(x * pi()) + cos(y)");
if (result.ok) {
  console.log(result.ast); // { type: "BinOp", value: "Add", children: [...] }
} else {
  console.error(result.error);
}
```

### Scalar code generation

```js
import { scalar } from "./pkg/dew_wasm.js";

const wgsl = scalar.emit_wgsl("clamp(x * 2, 0, 1)");
if (wgsl.ok) console.log(wgsl.code);

const glsl = scalar.emit_glsl("smoothstep(0, 1, t)");
const lua = scalar.emit_lua("lerp(a, b, t)");
```

### Linalg / complex / quaternion code generation

These take an extra `var_types` object mapping variable names to their
domain type, since the emitters need to know a variable's shape:

```js
import { linalg, complex, quaternion } from "./pkg/dew_wasm.js";

const wgsl = linalg.emit_wgsl_linalg("normalize(a + b)", {
  a: "vec3",
  b: "vec3",
});

const glsl = complex.emit_glsl_complex("conj(z) * z", { z: "complex" });

const lua = quaternion.emit_lua_quaternion("rotate(v, q)", {
  v: "vec3",
  q: "quaternion",
});
```

Each `emit_*` function returns `{ ok: true, code }` on success or
`{ ok: false, error }` on failure (parse error, unknown variable, or type
mismatch).

## Backends

WGSL, GLSL, and Lua (codegen only — no native `mlua` execution in WASM).
Cranelift, C, CUDA, HIP, and OpenCL are not exposed here, as they target
native code paths that don't apply in a browser.
