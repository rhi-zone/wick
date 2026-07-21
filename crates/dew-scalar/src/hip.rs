//! HIP code generation for scalar expressions.
//!
//! Compiles expression ASTs to HIP source code.
//! HIP is source-compatible with CUDA, using the same math functions
//! (sinf, cosf, etc.) and standard float literals.

use dew_cond::hip as cond;
use dew_core::{Ast, BinOp, UnaryOp};

/// HIP emission error.
#[derive(Debug, Clone, PartialEq)]
pub enum HIPError {
    /// Unknown function.
    UnknownFunction(String),
    /// Feature not supported in HIP codegen.
    UnsupportedFeature(String),
}

impl std::fmt::Display for HIPError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HIPError::UnknownFunction(name) => write!(f, "unknown function: '{name}'"),
            HIPError::UnsupportedFeature(feat) => {
                write!(f, "unsupported feature in HIP codegen: {feat}")
            }
        }
    }
}

impl std::error::Error for HIPError {}

/// Result of emitting HIP code.
#[derive(Debug, Clone)]
pub struct HIPExpr {
    /// The HIP expression string.
    pub code: String,
}

impl HIPExpr {
    pub fn new(code: impl Into<String>) -> Self {
        Self { code: code.into() }
    }
}

/// Mapping of dew functions to HIP functions.
enum HIPFunc {
    /// Single argument function: func(x)
    Func1(&'static str),
    /// Two argument function: func(x, y)
    Func2(&'static str),
    /// Three argument function: func(x, y, z)
    Func3(&'static str),
    /// Constant value
    Const(&'static str),
    /// tau = 2 * M_PI
    Tau,
    /// 1 / sqrt(x)
    InverseSqrt,
    /// sign(x) = copysignf(1.0f, x)
    Sign,
    /// fract(x) = x - floorf(x)
    Fract,
    /// clamp(x, lo, hi) = fminf(fmaxf(x, lo), hi)
    Clamp,
    /// lerp(a, b, t) = a + (b - a) * t
    Lerp,
    /// smoothstep(e0, e1, x)
    Smoothstep,
    /// step(edge, x) = x >= edge ? 1.0f : 0.0f
    Step,
    /// saturate(x) = fminf(fmaxf(x, 0.0f), 1.0f)
    Saturate,
}

/// Returns the HIP equivalent for a scalar function name.
fn hip_func_name(name: &str) -> Option<HIPFunc> {
    Some(match name {
        // Constants
        "pi" => HIPFunc::Const("M_PI"),
        "e" => HIPFunc::Const("M_E"),
        "tau" => HIPFunc::Tau,

        // Trig
        "sin" => HIPFunc::Func1("sinf"),
        "cos" => HIPFunc::Func1("cosf"),
        "tan" => HIPFunc::Func1("tanf"),
        "asin" => HIPFunc::Func1("asinf"),
        "acos" => HIPFunc::Func1("acosf"),
        "atan" => HIPFunc::Func1("atanf"),
        "atan2" => HIPFunc::Func2("atan2f"),
        "sinh" => HIPFunc::Func1("sinhf"),
        "cosh" => HIPFunc::Func1("coshf"),
        "tanh" => HIPFunc::Func1("tanhf"),
        "asinh" => HIPFunc::Func1("asinhf"),
        "acosh" => HIPFunc::Func1("acoshf"),
        "atanh" => HIPFunc::Func1("atanhf"),

        // Exp/log
        "exp" => HIPFunc::Func1("expf"),
        "exp2" => HIPFunc::Func1("exp2f"),
        "log" | "ln" => HIPFunc::Func1("logf"),
        "log2" => HIPFunc::Func1("log2f"),
        "log10" => HIPFunc::Func1("log10f"),
        "pow" => HIPFunc::Func2("powf"),
        "sqrt" => HIPFunc::Func1("sqrtf"),
        "inversesqrt" | "rsqrt" => HIPFunc::InverseSqrt,
        "cbrt" => HIPFunc::Func1("cbrtf"),

        // Common math
        "abs" => HIPFunc::Func1("fabsf"),
        "sign" => HIPFunc::Sign,
        "floor" => HIPFunc::Func1("floorf"),
        "ceil" => HIPFunc::Func1("ceilf"),
        "round" => HIPFunc::Func1("roundf"),
        "trunc" => HIPFunc::Func1("truncf"),
        "fract" => HIPFunc::Fract,
        "min" => HIPFunc::Func2("fminf"),
        "max" => HIPFunc::Func2("fmaxf"),
        "clamp" => HIPFunc::Clamp,
        "saturate" => HIPFunc::Saturate,

        // Interpolation
        "lerp" | "mix" => HIPFunc::Lerp,
        "step" => HIPFunc::Step,
        "smoothstep" => HIPFunc::Smoothstep,

        // Other
        "copysign" => HIPFunc::Func2("copysignf"),
        "fma" => HIPFunc::Func3("fmaf"),
        "hypot" => HIPFunc::Func2("hypotf"),

        _ => return None,
    })
}

/// Format a numeric literal for HIP.
fn format_literal(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{:.1}f", n)
    } else {
        format!("{}f", n)
    }
}

/// Emit HIP code for an expression AST.
pub fn emit_hip(ast: &Ast) -> Result<HIPExpr, HIPError> {
    match ast {
        Ast::Num(n) => Ok(HIPExpr::new(format_literal(*n))),

        Ast::Var(name) => Ok(HIPExpr::new(name.clone())),

        Ast::BinOp(op, left, right) => {
            let l = emit_hip(left)?;
            let r = emit_hip(right)?;
            let code = match op {
                BinOp::Add => format!("({} + {})", l.code, r.code),
                BinOp::Sub => format!("({} - {})", l.code, r.code),
                BinOp::Mul => format!("({} * {})", l.code, r.code),
                BinOp::Div => format!("({} / {})", l.code, r.code),
                BinOp::Pow => format!("powf({}, {})", l.code, r.code),
                BinOp::Rem => format!("fmodf({}, {})", l.code, r.code),
                BinOp::BitAnd | BinOp::BitOr | BinOp::Shl | BinOp::Shr => {
                    return Err(HIPError::UnsupportedFeature(
                        "bitwise ops on float".to_string(),
                    ));
                }
            };
            Ok(HIPExpr::new(code))
        }

        Ast::UnaryOp(op, inner) => {
            let inner = emit_hip(inner)?;
            let code = match op {
                UnaryOp::Neg => format!("(-{})", inner.code),
                UnaryOp::Not => {
                    let bool_expr = cond::scalar_to_bool(&inner.code);
                    cond::bool_to_scalar(&cond::emit_not(&bool_expr))
                }
                UnaryOp::BitNot => {
                    return Err(HIPError::UnsupportedFeature(
                        "bitwise not on float".to_string(),
                    ));
                }
            };
            Ok(HIPExpr::new(code))
        }

        Ast::Call(name, args) => {
            let arg_codes: Vec<String> = args
                .iter()
                .map(|a| emit_hip(a).map(|e| e.code))
                .collect::<Result<_, _>>()?;

            let func =
                hip_func_name(name).ok_or_else(|| HIPError::UnknownFunction(name.clone()))?;

            let code = match func {
                HIPFunc::Func1(f) => format!("{}({})", f, arg_codes[0]),
                HIPFunc::Func2(f) => format!("{}({}, {})", f, arg_codes[0], arg_codes[1]),
                HIPFunc::Func3(f) => {
                    format!(
                        "{}({}, {}, {})",
                        f, arg_codes[0], arg_codes[1], arg_codes[2]
                    )
                }
                HIPFunc::Const(c) => c.to_string(),
                HIPFunc::Tau => "(2.0f * M_PI)".to_string(),
                HIPFunc::InverseSqrt => format!("rsqrtf({})", arg_codes[0]),
                HIPFunc::Sign => format!("copysignf(1.0f, {})", arg_codes[0]),
                HIPFunc::Fract => format!("({} - floorf({}))", arg_codes[0], arg_codes[0]),
                HIPFunc::Clamp => format!(
                    "fminf(fmaxf({}, {}), {})",
                    arg_codes[0], arg_codes[1], arg_codes[2]
                ),
                HIPFunc::Lerp => format!(
                    "({} + ({} - {}) * {})",
                    arg_codes[0], arg_codes[1], arg_codes[0], arg_codes[2]
                ),
                HIPFunc::Step => format!("({} >= {} ? 1.0f : 0.0f)", arg_codes[1], arg_codes[0]),
                HIPFunc::Smoothstep => format!(
                    "smoothstep_impl({}, {}, {})",
                    arg_codes[0], arg_codes[1], arg_codes[2]
                ),
                HIPFunc::Saturate => format!("fminf(fmaxf({}, 0.0f), 1.0f)", arg_codes[0]),
            };
            Ok(HIPExpr::new(code))
        }

        Ast::Compare(op, left, right) => {
            let l = emit_hip(left)?;
            let r = emit_hip(right)?;
            let bool_expr = cond::emit_compare(*op, &l.code, &r.code);
            Ok(HIPExpr::new(cond::bool_to_scalar(&bool_expr)))
        }

        Ast::And(left, right) => {
            let l = emit_hip(left)?;
            let r = emit_hip(right)?;
            let l_bool = cond::scalar_to_bool(&l.code);
            let r_bool = cond::scalar_to_bool(&r.code);
            let bool_expr = cond::emit_and(&l_bool, &r_bool);
            Ok(HIPExpr::new(cond::bool_to_scalar(&bool_expr)))
        }

        Ast::Or(left, right) => {
            let l = emit_hip(left)?;
            let r = emit_hip(right)?;
            let l_bool = cond::scalar_to_bool(&l.code);
            let r_bool = cond::scalar_to_bool(&r.code);
            let bool_expr = cond::emit_or(&l_bool, &r_bool);
            Ok(HIPExpr::new(cond::bool_to_scalar(&bool_expr)))
        }

        Ast::If(cond_ast, then_ast, else_ast) => {
            let c = emit_hip(cond_ast)?;
            let t = emit_hip(then_ast)?;
            let e = emit_hip(else_ast)?;
            let cond_bool = cond::scalar_to_bool(&c.code);
            Ok(HIPExpr::new(cond::emit_if(&cond_bool, &t.code, &e.code)))
        }

        Ast::Let { name, value, body } => {
            // For simple emission, we inline the let binding
            // For proper function emission with statements, use emit_hip_fn
            let val = emit_hip(value)?;
            let body_code = emit_hip(body)?.code;
            // Replace variable references in body - simple approach
            Ok(HIPExpr::new(
                body_code.replace(name, &format!("({})", val.code)),
            ))
        }
    }
}

/// Emit a complete HIP device function.
pub fn emit_hip_fn(name: &str, ast: &Ast, params: &[&str]) -> Result<String, HIPError> {
    let params_str = params
        .iter()
        .map(|n| format!("float {}", n))
        .collect::<Vec<_>>()
        .join(", ");

    let expr = emit_hip(ast)?;

    Ok(format!(
        "__device__ float {}({}) {{\n    return {};\n}}",
        name, params_str, expr.code
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dew_core::Expr;

    fn emit(expr: &str) -> Result<HIPExpr, HIPError> {
        let expr = Expr::parse(expr).unwrap();
        emit_hip(expr.ast())
    }

    #[test]
    fn test_literal() {
        let result = emit("3.14").unwrap();
        assert_eq!(result.code, "3.14f");
    }

    #[test]
    fn test_variable() {
        let result = emit("x").unwrap();
        assert_eq!(result.code, "x");
    }

    #[test]
    fn test_binop() {
        let result = emit("a + b").unwrap();
        assert_eq!(result.code, "(a + b)");
    }

    #[test]
    fn test_pow() {
        let result = emit("a ^ b").unwrap();
        assert_eq!(result.code, "powf(a, b)");
    }

    #[test]
    fn test_sin() {
        let result = emit("sin(x)").unwrap();
        assert_eq!(result.code, "sinf(x)");
    }

    #[test]
    fn test_sqrt() {
        let result = emit("sqrt(x)").unwrap();
        assert_eq!(result.code, "sqrtf(x)");
    }

    #[test]
    fn test_rsqrt() {
        let result = emit("rsqrt(x)").unwrap();
        assert_eq!(result.code, "rsqrtf(x)");
    }

    #[test]
    fn test_clamp() {
        let result = emit("clamp(x, a, b)").unwrap();
        assert!(result.code.contains("fminf"));
        assert!(result.code.contains("fmaxf"));
    }

    #[test]
    fn test_lerp() {
        let result = emit("lerp(a, b, t)").unwrap();
        assert!(result.code.contains("+"));
        assert!(result.code.contains("-"));
    }

    #[test]
    fn test_conditional() {
        let result = emit("if x > 0.0 then 1.0 else -1.0").unwrap();
        assert!(result.code.contains("?"));
        assert!(result.code.contains(":"));
    }

    #[test]
    fn test_emit_hip_fn() {
        let expr = Expr::parse("sin(x) + cos(y)").unwrap();
        let code = emit_hip_fn("compute", expr.ast(), &["x", "y"]).unwrap();
        assert!(code.contains("__device__"));
        assert!(code.contains("sinf"));
        assert!(code.contains("cosf"));
    }

    #[test]
    fn test_constants() {
        let result = emit("pi()").unwrap();
        assert_eq!(result.code, "M_PI");
    }
}
