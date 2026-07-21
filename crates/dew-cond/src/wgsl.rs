//! WGSL code generation helpers for conditionals.

use dew_core::CompareOp;

/// Emit WGSL code for a comparison operation.
/// Returns boolean expression as string.
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

/// Emit WGSL code for logical AND.
/// Inputs are boolean expressions.
pub fn emit_and(left: &str, right: &str) -> String {
    format!("({} && {})", left, right)
}

/// Emit WGSL code for logical OR.
/// Inputs are boolean expressions.
pub fn emit_or(left: &str, right: &str) -> String {
    format!("({} || {})", left, right)
}

/// Emit WGSL code for logical NOT.
/// Input is a boolean expression.
pub fn emit_not(expr: &str) -> String {
    format!("(!{})", expr)
}

/// Emit WGSL code for a conditional (if/then/else).
/// Uses WGSL's select() function.
/// `cond` should be a boolean expression.
pub fn emit_if(cond: &str, then_expr: &str, else_expr: &str) -> String {
    // WGSL select: select(false_val, true_val, condition)
    format!("select({}, {}, {})", else_expr, then_expr, cond)
}

/// Convert a scalar (f32) expression to boolean for use in conditions.
/// Non-zero is true, zero is false.
pub fn scalar_to_bool(expr: &str) -> String {
    format!("({} != 0.0)", expr)
}

/// Convert a boolean expression to scalar (f32).
/// true -> 1.0, false -> 0.0
pub fn bool_to_scalar(expr: &str) -> String {
    format!("select(0.0, 1.0, {})", expr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_compare() {
        assert_eq!(emit_compare(CompareOp::Lt, "a", "b"), "(a < b)");
        assert_eq!(emit_compare(CompareOp::Le, "x", "5.0"), "(x <= 5.0)");
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
            "select(else_val, then_val, cond)"
        );
    }

    #[test]
    fn test_conversions() {
        assert_eq!(scalar_to_bool("x"), "(x != 0.0)");
        assert_eq!(bool_to_scalar("cond"), "select(0.0, 1.0, cond)");
    }
}
