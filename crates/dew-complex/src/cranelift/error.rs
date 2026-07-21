//! Error types for Cranelift compilation.

use crate::Type;

/// Error during Cranelift compilation.
#[derive(Debug, Clone)]
pub enum CraneliftError {
    UnknownVariable(String),
    UnknownFunction(String),
    TypeMismatch {
        op: &'static str,
        left: Type,
        right: Type,
    },
    UnsupportedReturnType(Type),
    JitError(String),
    /// Conditionals require scalar types.
    UnsupportedConditional(&'static str),
}

impl std::fmt::Display for CraneliftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CraneliftError::UnknownVariable(name) => write!(f, "unknown variable: '{name}'"),
            CraneliftError::UnknownFunction(name) => write!(f, "unknown function: '{name}'"),
            CraneliftError::TypeMismatch { op, left, right } => {
                write!(f, "type mismatch for {op}: {left} vs {right}")
            }
            CraneliftError::UnsupportedReturnType(t) => {
                write!(f, "unsupported return type: {t}")
            }
            CraneliftError::JitError(msg) => write!(f, "JIT error: {msg}"),
            CraneliftError::UnsupportedConditional(what) => {
                write!(
                    f,
                    "conditionals not supported in complex cranelift backend: {what}"
                )
            }
        }
    }
}

impl std::error::Error for CraneliftError {}
