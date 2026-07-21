//! Cranelift JIT compilation for quaternion expressions.
//!
//! Compiles typed expressions to native code via Cranelift.
//!
//! # Representation
//!
//! - Scalar: single f32
//! - Vec3: three f32 values (x, y, z)
//! - Quaternion: four f32 values (x, y, z, w)

/// Dispatch a JIT function call based on parameter count.
/// Centralizes the unsafe transmute logic for all arities 0-16.
macro_rules! jit_call {
    ($func_ptr:expr, $args:expr, $ret:ty, []) => {{
        let f: extern "C" fn() -> $ret = std::mem::transmute($func_ptr);
        f()
    }};
    ($func_ptr:expr, $args:expr, $ret:ty, [$($idx:tt),+]) => {{
        let f: extern "C" fn($(jit_call!(@ty $idx)),+) -> $ret = std::mem::transmute($func_ptr);
        f($($args[$idx]),+)
    }};
    (@ty $idx:tt) => { f32 };
}

/// Dispatch a JIT function call with an output pointer parameter.
/// The function signature is `fn(args..., *mut f32) -> ()`.
macro_rules! jit_call_outptr {
    ($func_ptr:expr, $args:expr, $out_ptr:expr, []) => {{
        let f: extern "C" fn(*mut f32) = std::mem::transmute($func_ptr);
        f($out_ptr)
    }};
    ($func_ptr:expr, $args:expr, $out_ptr:expr, [$($idx:tt),+]) => {{
        let f: extern "C" fn($(jit_call_outptr!(@ty $idx),)+ *mut f32) = std::mem::transmute($func_ptr);
        f($($args[$idx],)+ $out_ptr)
    }};
    (@ty $idx:tt) => { f32 };
}

mod compiled;
mod error;
mod jit;
mod types;

// Public re-exports
pub use compiled::{CompiledQuatFn, CompiledQuaternionFn, CompiledVec3Fn};
pub use error::CraneliftError;
pub use jit::QuaternionJit;
pub use types::{TypedValue, VarSpec};
