//! Lua code generation helpers for conditionals.

use dew_core::CompareOp;

/// Emit Lua code for a comparison operation.
/// Returns a Lua expression that evaluates to a boolean.
pub fn emit_compare(op: CompareOp, left: &str, right: &str) -> String {
    let op_str = match op {
        CompareOp::Lt => "<",
        CompareOp::Le => "<=",
        CompareOp::Gt => ">",
        CompareOp::Ge => ">=",
        CompareOp::Eq => "==",
        CompareOp::Ne => "~=",
    };
    format!("({} {} {})", left, op_str, right)
}

/// Emit Lua code for logical AND.
pub fn emit_and(left: &str, right: &str) -> String {
    format!("({} and {})", left, right)
}

/// Emit Lua code for logical OR.
pub fn emit_or(left: &str, right: &str) -> String {
    format!("({} or {})", left, right)
}

/// Emit Lua code for logical NOT.
pub fn emit_not(expr: &str) -> String {
    format!("(not {})", expr)
}

/// Emit Lua code for a conditional (if/then/else).
/// Uses Lua's ternary idiom: cond and then_val or else_val
/// Note: This works correctly when then_val is never false/nil.
/// For numeric expressions this is always safe.
pub fn emit_if(cond: &str, then_expr: &str, else_expr: &str) -> String {
    format!("({} and {} or {})", cond, then_expr, else_expr)
}

/// Convert a scalar expression to boolean for use in conditions.
/// In Lua, 0 is truthy, so we need explicit comparison.
pub fn scalar_to_bool(expr: &str) -> String {
    format!("({} ~= 0)", expr)
}

/// Convert a boolean expression to scalar.
/// true -> 1.0, false -> 0.0
pub fn bool_to_scalar(expr: &str) -> String {
    format!("({} and 1.0 or 0.0)", expr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_compare() {
        assert_eq!(emit_compare(CompareOp::Lt, "a", "b"), "(a < b)");
        assert_eq!(emit_compare(CompareOp::Ne, "x", "5"), "(x ~= 5)");
    }

    #[test]
    fn test_emit_logic() {
        assert_eq!(emit_and("a", "b"), "(a and b)");
        assert_eq!(emit_or("a", "b"), "(a or b)");
        assert_eq!(emit_not("a"), "(not a)");
    }

    #[test]
    fn test_emit_if() {
        assert_eq!(
            emit_if("cond", "then_val", "else_val"),
            "(cond and then_val or else_val)"
        );
    }
}
