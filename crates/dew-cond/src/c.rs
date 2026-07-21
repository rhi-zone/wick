//! C code generation helpers for conditionals.

use dew_core::CompareOp;

/// Emit C code for a comparison operation.
/// Returns boolean expression as string (evaluates to 0 or 1 in C).
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

/// Emit C code for logical AND.
/// Inputs are boolean expressions.
pub fn emit_and(left: &str, right: &str) -> String {
    format!("({} && {})", left, right)
}

/// Emit C code for logical OR.
/// Inputs are boolean expressions.
pub fn emit_or(left: &str, right: &str) -> String {
    format!("({} || {})", left, right)
}

/// Emit C code for logical NOT.
/// Input is a boolean expression.
pub fn emit_not(expr: &str) -> String {
    format!("(!{})", expr)
}

/// Emit C code for a conditional (if/then/else).
/// Uses C's ternary operator.
/// `cond` should be a boolean expression.
pub fn emit_if(cond: &str, then_expr: &str, else_expr: &str) -> String {
    format!("({} ? {} : {})", cond, then_expr, else_expr)
}

/// Convert a scalar (float) expression to boolean for use in conditions.
/// Non-zero is true, zero is false.
pub fn scalar_to_bool(expr: &str) -> String {
    format!("({} != 0.0f)", expr)
}

/// Convert a boolean expression to scalar (float).
/// true -> 1.0f, false -> 0.0f
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
    fn test_conversions() {
        assert_eq!(scalar_to_bool("x"), "(x != 0.0f)");
        assert_eq!(bool_to_scalar("cond"), "(cond ? 1.0f : 0.0f)");
    }
}
