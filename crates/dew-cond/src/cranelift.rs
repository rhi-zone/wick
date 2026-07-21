//! Cranelift code generation helpers for conditionals.

use cranelift_codegen::ir::condcodes::FloatCC;
use cranelift_codegen::ir::{InstBuilder, Value};
use cranelift_frontend::FunctionBuilder;
use dew_core::CompareOp;

/// Emit Cranelift IR for a comparison operation.
/// Returns a boolean value (i8).
pub fn emit_compare(
    builder: &mut FunctionBuilder,
    op: CompareOp,
    left: Value,
    right: Value,
) -> Value {
    let cc = match op {
        CompareOp::Lt => FloatCC::LessThan,
        CompareOp::Le => FloatCC::LessThanOrEqual,
        CompareOp::Gt => FloatCC::GreaterThan,
        CompareOp::Ge => FloatCC::GreaterThanOrEqual,
        CompareOp::Eq => FloatCC::Equal,
        CompareOp::Ne => FloatCC::NotEqual,
    };
    builder.ins().fcmp(cc, left, right)
}

/// Emit Cranelift IR for logical AND.
/// Inputs are boolean values (i8). Returns boolean.
pub fn emit_and(builder: &mut FunctionBuilder, left: Value, right: Value) -> Value {
    builder.ins().band(left, right)
}

/// Emit Cranelift IR for logical OR.
/// Inputs are boolean values (i8). Returns boolean.
pub fn emit_or(builder: &mut FunctionBuilder, left: Value, right: Value) -> Value {
    builder.ins().bor(left, right)
}

/// Emit Cranelift IR for logical NOT.
/// Input is a boolean value (i8). Returns boolean.
pub fn emit_not(builder: &mut FunctionBuilder, value: Value) -> Value {
    let one = builder.ins().iconst(cranelift_codegen::ir::types::I8, 1);
    builder.ins().bxor(value, one)
}

/// Emit Cranelift IR for a conditional (if/then/else).
/// `cond` is a boolean value (i8).
/// `then_val` and `else_val` are f32 values.
/// Returns f32.
pub fn emit_if(
    builder: &mut FunctionBuilder,
    cond: Value,
    then_val: Value,
    else_val: Value,
) -> Value {
    builder.ins().select(cond, then_val, else_val)
}

/// Convert a scalar (f32) value to boolean (i8).
/// Non-zero is true (1), zero is false (0).
pub fn scalar_to_bool(builder: &mut FunctionBuilder, value: Value) -> Value {
    let zero = builder.ins().f32const(0.0);
    builder.ins().fcmp(FloatCC::NotEqual, value, zero)
}

/// Convert a boolean (i8) value to scalar (f32).
/// true (non-zero) -> 1.0, false (0) -> 0.0
pub fn bool_to_scalar(builder: &mut FunctionBuilder, value: Value) -> Value {
    let zero = builder.ins().f32const(0.0);
    let one = builder.ins().f32const(1.0);
    builder.ins().select(value, one, zero)
}

#[cfg(test)]
mod tests {
    // Cranelift tests require more setup, tested via integration tests
}
