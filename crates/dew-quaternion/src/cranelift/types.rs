//! Type definitions for Cranelift compilation.

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
pub(super) extern "C" fn math_acos(x: f32) -> f32 {
    x.acos()
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
            name: "quat_sqrt",
            ptr: math_sqrt as *const u8,
        },
        MathSymbol {
            name: "quat_pow",
            ptr: math_pow as *const u8,
        },
        MathSymbol {
            name: "quat_acos",
            ptr: math_acos as *const u8,
        },
        MathSymbol {
            name: "quat_sin",
            ptr: math_sin as *const u8,
        },
        MathSymbol {
            name: "quat_cos",
            ptr: math_cos as *const u8,
        },
    ]
}

// ============================================================================
// Typed values during compilation
// ============================================================================

/// A typed value during compilation.
#[derive(Clone)]
pub enum TypedValue {
    Scalar(CraneliftValue),
    Vec3([CraneliftValue; 3]),
    Quaternion([CraneliftValue; 4]),
}

impl TypedValue {
    pub(super) fn typ(&self) -> Type {
        match self {
            TypedValue::Scalar(_) => Type::Scalar,
            TypedValue::Vec3(_) => Type::Vec3,
            TypedValue::Quaternion(_) => Type::Quaternion,
        }
    }

    pub(super) fn as_scalar(&self) -> Option<CraneliftValue> {
        match self {
            TypedValue::Scalar(v) => Some(*v),
            _ => None,
        }
    }

    pub(super) fn as_vec3(&self) -> Option<[CraneliftValue; 3]> {
        match self {
            TypedValue::Vec3(v) => Some(*v),
            _ => None,
        }
    }

    pub(super) fn as_quaternion(&self) -> Option<[CraneliftValue; 4]> {
        match self {
            TypedValue::Quaternion(v) => Some(*v),
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
            Type::Vec3 => 3,
            Type::Quaternion => 4,
        }
    }
}

// ============================================================================
// Math functions struct
// ============================================================================

#[allow(dead_code)]
pub(super) struct MathFuncs {
    pub sqrt: FuncRef,
    pub pow: FuncRef,
    pub acos: FuncRef,
    pub sin: FuncRef,
    pub cos: FuncRef,
}
