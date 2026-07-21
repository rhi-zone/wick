//! OpenCL code generation helpers for conditionals.
//!
//! OpenCL uses C-like syntax with some differences:
//! - Float literals can omit `f` suffix (single precision is default in kernels)
//! - `select(else_val, then_val, cond)` for vectorized conditionals
//! - Standard C ternary operator for scalar conditionals

use dew_core::CompareOp;

/// Emit OpenCL code for a comparison operation.
/// Returns boolean expression (evaluates to 0 or 1).
pub fn emit_compare(op: CompareOp, left: &str, right: &str) -> String {
    let op_str = match op {
        CompareOp::Lt => "<",
        CompareOp::Le => "<=",
        CompareOp::Gt => ">",
        CompareOp::Ge => ">=",
        CompareOp::Eq => "==",
        CompareOp::Ne => "!=",
    };
    format!("({} {} {})", left, op_str, right)
}

/// Emit OpenCL code for logical AND.
pub fn emit_and(left: &str, right: &str) -> String {
    format!("({} && {})", left, right)
}

/// Emit OpenCL code for logical OR.
pub fn emit_or(left: &str, right: &str) -> String {
    format!("({} || {})", left, right)
}

/// Emit OpenCL code for logical NOT.
pub fn emit_not(expr: &str) -> String {
    format!("(!{})", expr)
}

/// Emit OpenCL code for a scalar conditional (if/then/else).
/// Uses ternary operator for scalars.
pub fn emit_if(cond: &str, then_expr: &str, else_expr: &str) -> String {
    format!("({} ? {} : {})", cond, then_expr, else_expr)
}

/// Emit OpenCL code for a vectorized conditional.
/// Uses OpenCL's select() built-in: select(else_val, then_val, cond)
/// Note: condition must be an integer type (use comparison result directly).
pub fn emit_select(cond: &str, then_expr: &str, else_expr: &str) -> String {
    format!("select({}, {}, {})", else_expr, then_expr, cond)
}

/// Convert a scalar (float) expression to boolean for use in conditions.
pub fn scalar_to_bool(expr: &str) -> String {
    format!("({} != 0.0f)", expr)
}

/// Convert a boolean expression to scalar (float).
pub fn bool_to_scalar(expr: &str) -> String {
    format!("({} ? 1.0f : 0.0f)", expr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_compare() {
        assert_eq!(emit_compare(CompareOp::Lt, "a", "b"), "(a < b)");
        assert_eq!(emit_compare(CompareOp::Le, "x", "5.0f"), "(x <= 5.0f)");
        assert_eq!(emit_compare(CompareOp::Eq, "a", "b"), "(a == b)");
    }

    #[test]
    fn test_emit_logic() {
        assert_eq!(emit_and("a", "b"), "(a && b)");
        assert_eq!(emit_or("a", "b"), "(a || b)");
        assert_eq!(emit_not("a"), "(!a)");
    }

    #[test]
    fn test_emit_if() {
        assert_eq!(
            emit_if("cond", "then_val", "else_val"),
            "(cond ? then_val : else_val)"
        );
    }

    #[test]
    fn test_emit_select() {
        // select(else, then, cond) - OpenCL argument order
        assert_eq!(
            emit_select("cond", "then_val", "else_val"),
            "select(else_val, then_val, cond)"
        );
    }

    #[test]
    fn test_conversions() {
        assert_eq!(scalar_to_bool("x"), "(x != 0.0f)");
        assert_eq!(bool_to_scalar("cond"), "(cond ? 1.0f : 0.0f)");
    }
}
