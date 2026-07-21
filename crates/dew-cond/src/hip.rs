//! HIP conditional code generation helpers.
//!
//! HIP uses standard C-style ternary operator for scalar conditionals.
//! For vector operations, explicit component selection may be needed.
//! HIP is source-compatible with CUDA syntax.

use dew_core::CompareOp;

/// Emit a comparison expression.
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

/// Emit a conditional expression (ternary operator).
pub fn emit_if(cond: &str, then_expr: &str, else_expr: &str) -> String {
    format!("({} ? {} : {})", cond, then_expr, else_expr)
}

/// Emit logical AND.
pub fn emit_and(left: &str, right: &str) -> String {
    format!("({} && {})", left, right)
}

/// Emit logical OR.
pub fn emit_or(left: &str, right: &str) -> String {
    format!("({} || {})", left, right)
}

/// Emit logical NOT.
pub fn emit_not(expr: &str) -> String {
    format!("(!{})", expr)
}

/// Convert a scalar float to bool.
pub fn scalar_to_bool(expr: &str) -> String {
    format!("({} != 0.0f)", expr)
}

/// Convert a bool expression to float (1.0 or 0.0).
pub fn bool_to_scalar(expr: &str) -> String {
    format!("({} ? 1.0f : 0.0f)", expr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare() {
        assert_eq!(emit_compare(CompareOp::Lt, "a", "b"), "(a < b)");
        assert_eq!(emit_compare(CompareOp::Ge, "x", "y"), "(x >= y)");
    }

    #[test]
    fn test_if() {
        assert_eq!(emit_if("cond", "a", "b"), "(cond ? a : b)");
    }

    #[test]
    fn test_logical() {
        assert_eq!(emit_and("a", "b"), "(a && b)");
        assert_eq!(emit_or("a", "b"), "(a || b)");
        assert_eq!(emit_not("x"), "(!x)");
    }

    #[test]
    fn test_conversions() {
        assert_eq!(scalar_to_bool("x"), "(x != 0.0f)");
        assert_eq!(bool_to_scalar("cond"), "(cond ? 1.0f : 0.0f)");
    }
}
