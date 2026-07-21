//! Type definitions for Cranelift compilation.

use crate::Type;
use cranelift_codegen::ir::{FuncRef, Value as CraneliftValue};

/// A typed value during compilation.
#[derive(Clone)]
pub enum TypedValue {
    Scalar(CraneliftValue),
    Complex([CraneliftValue; 2]),
}

impl TypedValue {
    pub(super) fn typ(&self) -> Type {
        match self {
            TypedValue::Scalar(_) => Type::Scalar,
            TypedValue::Complex(_) => Type::Complex,
        }
    }

    pub(super) fn as_scalar(&self) -> Option<CraneliftValue> {
        match self {
            TypedValue::Scalar(v) => Some(*v),
            _ => None,
        }
    }

    pub(super) fn as_complex(&self) -> Option<[CraneliftValue; 2]> {
        match self {
            TypedValue::Complex(v) => Some(*v),
            _ => None,
        }
    }
}

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
            Type::Complex => 2,
        }
    }
}

// ============================================================================
// Math function infrastructure
// ============================================================================

pub(super) struct MathSymbol {
    pub name: &'static str,
    pub ptr: *const u8,
}

pub(super) fn math_symbols() -> Vec<MathSymbol> {
    vec![
        MathSymbol {
            name: "complex_sqrt",
            ptr: math_sqrt as *const u8,
        },
        MathSymbol {
            name: "complex_pow",
            ptr: math_pow as *const u8,
        },
        MathSymbol {
            name: "complex_sin",
            ptr: math_sin as *const u8,
        },
        MathSymbol {
            name: "complex_cos",
            ptr: math_cos as *const u8,
        },
        MathSymbol {
            name: "complex_exp",
            ptr: math_exp as *const u8,
        },
        MathSymbol {
            name: "complex_log",
            ptr: math_log as *const u8,
        },
        MathSymbol {
            name: "complex_atan2",
            ptr: math_atan2 as *const u8,
        },
    ]
}

pub(super) struct MathFuncs {
    pub sqrt: FuncRef,
    pub pow: FuncRef,
    pub atan2: FuncRef,
    pub sin: FuncRef,
    pub cos: FuncRef,
    pub exp: FuncRef,
    pub log: FuncRef,
}

// Math function wrappers
extern "C" fn math_sqrt(x: f32) -> f32 {
    x.sqrt()
}
extern "C" fn math_pow(base: f32, exp: f32) -> f32 {
    base.powf(exp)
}
extern "C" fn math_sin(x: f32) -> f32 {
    x.sin()
}
extern "C" fn math_cos(x: f32) -> f32 {
    x.cos()
}
extern "C" fn math_exp(x: f32) -> f32 {
    x.exp()
}
extern "C" fn math_log(x: f32) -> f32 {
    x.ln()
}
extern "C" fn math_atan2(y: f32, x: f32) -> f32 {
    y.atan2(x)
}
