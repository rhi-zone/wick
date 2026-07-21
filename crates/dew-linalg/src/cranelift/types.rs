//! Types and helpers for Cranelift JIT compilation.

use crate::Type;
use cranelift_codegen::ir::{FuncRef, Value as CraneliftValue};

// ============================================================================
// Math function wrappers
// ============================================================================

pub(super) extern "C" fn math_sqrt(x: f32) -> f32 {
    x.sqrt()
}
pub(super) extern "C" fn math_pow(base: f32, exp: f32) -> f32 {
    base.powf(exp)
}
pub(super) extern "C" fn math_sin(x: f32) -> f32 {
    x.sin()
}
pub(super) extern "C" fn math_cos(x: f32) -> f32 {
    x.cos()
}

pub(super) struct MathSymbol {
    pub name: &'static str,
    pub ptr: *const u8,
}

pub(super) fn math_symbols() -> Vec<MathSymbol> {
    vec![
        MathSymbol {
            name: "linalg_sqrt",
            ptr: math_sqrt as *const u8,
        },
        MathSymbol {
            name: "linalg_pow",
            ptr: math_pow as *const u8,
        },
        MathSymbol {
            name: "linalg_sin",
            ptr: math_sin as *const u8,
        },
        MathSymbol {
            name: "linalg_cos",
            ptr: math_cos as *const u8,
        },
    ]
}

// ============================================================================
// Typed values during compilation
// ============================================================================

/// A typed value during compilation.
/// Scalars are single CraneliftValue, vectors are multiple CraneliftValues.
#[derive(Clone)]
pub enum TypedValue {
    Scalar(CraneliftValue),
    Vec2([CraneliftValue; 2]),
    #[cfg(feature = "3d")]
    Vec3([CraneliftValue; 3]),
    #[cfg(feature = "4d")]
    Vec4([CraneliftValue; 4]),
    /// Mat2 stored as [c0r0, c0r1, c1r0, c1r1] (column-major)
    Mat2([CraneliftValue; 4]),
    /// Mat3 stored as 9 values (column-major)
    #[cfg(feature = "3d")]
    Mat3([CraneliftValue; 9]),
    /// Mat4 stored as 16 values (column-major)
    #[cfg(feature = "4d")]
    Mat4([CraneliftValue; 16]),
}

impl TypedValue {
    pub(super) fn typ(&self) -> Type {
        match self {
            TypedValue::Scalar(_) => Type::Scalar,
            TypedValue::Vec2(_) => Type::Vec2,
            #[cfg(feature = "3d")]
            TypedValue::Vec3(_) => Type::Vec3,
            #[cfg(feature = "4d")]
            TypedValue::Vec4(_) => Type::Vec4,
            TypedValue::Mat2(_) => Type::Mat2,
            #[cfg(feature = "3d")]
            TypedValue::Mat3(_) => Type::Mat3,
            #[cfg(feature = "4d")]
            TypedValue::Mat4(_) => Type::Mat4,
        }
    }

    pub(super) fn as_scalar(&self) -> Option<CraneliftValue> {
        match self {
            TypedValue::Scalar(v) => Some(*v),
            _ => None,
        }
    }

    pub(super) fn as_vec2(&self) -> Option<[CraneliftValue; 2]> {
        match self {
            TypedValue::Vec2(v) => Some(*v),
            _ => None,
        }
    }

    #[cfg(feature = "3d")]
    pub(super) fn as_vec3(&self) -> Option<[CraneliftValue; 3]> {
        match self {
            TypedValue::Vec3(v) => Some(*v),
            _ => None,
        }
    }

    #[cfg(feature = "4d")]
    #[allow(dead_code)]
    pub(super) fn as_vec4(&self) -> Option<[CraneliftValue; 4]> {
        match self {
            TypedValue::Vec4(v) => Some(*v),
            _ => None,
        }
    }

    pub(super) fn as_mat2(&self) -> Option<[CraneliftValue; 4]> {
        match self {
            TypedValue::Mat2(m) => Some(*m),
            _ => None,
        }
    }

    #[cfg(feature = "3d")]
    pub(super) fn as_mat3(&self) -> Option<[CraneliftValue; 9]> {
        match self {
            TypedValue::Mat3(m) => Some(*m),
            _ => None,
        }
    }

    #[cfg(feature = "4d")]
    pub(super) fn as_mat4(&self) -> Option<[CraneliftValue; 16]> {
        match self {
            TypedValue::Mat4(m) => Some(*m),
            _ => None,
        }
    }
}

// ============================================================================
// Variable specification
// ============================================================================

/// Specification of a variable with its type.
#[derive(Debug, Clone)]
pub struct VarSpec {
    pub name: String,
    pub typ: Type,
}

impl VarSpec {
    pub fn new(name: impl Into<String>, typ: Type) -> Self {
        Self {
            name: name.into(),
            typ,
        }
    }

    /// Number of f32 parameters this variable needs.
    pub fn param_count(&self) -> usize {
        match self.typ {
            Type::Scalar => 1,
            Type::Vec2 => 2,
            #[cfg(feature = "3d")]
            Type::Vec3 => 3,
            #[cfg(feature = "4d")]
            Type::Vec4 => 4,
            Type::Mat2 => 4,
            #[cfg(feature = "3d")]
            Type::Mat3 => 9,
            #[cfg(feature = "4d")]
            Type::Mat4 => 16,
        }
    }
}

// ============================================================================
// Math function references
// ============================================================================

pub(super) struct MathFuncs {
    pub sqrt: FuncRef,
    pub pow: FuncRef,
    pub sin: FuncRef,
    pub cos: FuncRef,
}
