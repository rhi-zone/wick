//! CUDA code generation for scalar expressions.
//!
//! Compiles expression ASTs to CUDA source code.
//! Uses CUDA math functions (sinf, cosf, etc.) and standard float literals.

use dew_cond::cuda as cond;
use dew_core::{Ast, BinOp, UnaryOp};

/// CUDA emission error.
#[derive(Debug, Clone, PartialEq)]
pub enum CUDAError {
    /// Unknown function.
    UnknownFunction(String),
    /// Feature not supported in CUDA codegen.
    UnsupportedFeature(String),
}

impl std::fmt::Display for CUDAError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CUDAError::UnknownFunction(name) => write!(f, "unknown function: '{name}'"),
            CUDAError::UnsupportedFeature(feat) => {
                write!(f, "unsupported feature in CUDA codegen: {feat}")
            }
        }
    }
}

impl std::error::Error for CUDAError {}

/// Result of emitting CUDA code.
#[derive(Debug, Clone)]
pub struct CUDAExpr {
    /// The CUDA expression string.
    pub code: String,
}

impl CUDAExpr {
    pub fn new(code: impl Into<String>) -> Self {
        Self { code: code.into() }
    }
}

/// Mapping of dew functions to CUDA functions.
enum CUDAFunc {
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

/// Returns the CUDA equivalent for a scalar function name.
fn cuda_func_name(name: &str) -> Option<CUDAFunc> {
    Some(match name {
        // Constants
        "pi" => CUDAFunc::Const("M_PI"),
        "e" => CUDAFunc::Const("M_E"),
        "tau" => CUDAFunc::Tau,

        // Trig
        "sin" => CUDAFunc::Func1("sinf"),
        "cos" => CUDAFunc::Func1("cosf"),
        "tan" => CUDAFunc::Func1("tanf"),
        "asin" => CUDAFunc::Func1("asinf"),
        "acos" => CUDAFunc::Func1("acosf"),
        "atan" => CUDAFunc::Func1("atanf"),
        "atan2" => CUDAFunc::Func2("atan2f"),
        "sinh" => CUDAFunc::Func1("sinhf"),
        "cosh" => CUDAFunc::Func1("coshf"),
        "tanh" => CUDAFunc::Func1("tanhf"),
        "asinh" => CUDAFunc::Func1("asinhf"),
        "acosh" => CUDAFunc::Func1("acoshf"),
        "atanh" => CUDAFunc::Func1("atanhf"),

        // Exp/log
        "exp" => CUDAFunc::Func1("expf"),
        "exp2" => CUDAFunc::Func1("exp2f"),
        "log" | "ln" => CUDAFunc::Func1("logf"),
        "log2" => CUDAFunc::Func1("log2f"),
        "log10" => CUDAFunc::Func1("log10f"),
        "pow" => CUDAFunc::Func2("powf"),
        "sqrt" => CUDAFunc::Func1("sqrtf"),
        "inversesqrt" | "rsqrt" => CUDAFunc::InverseSqrt,
        "cbrt" => CUDAFunc::Func1("cbrtf"),

        // Common math
        "abs" => CUDAFunc::Func1("fabsf"),
        "sign" => CUDAFunc::Sign,
        "floor" => CUDAFunc::Func1("floorf"),
        "ceil" => CUDAFunc::Func1("ceilf"),
        "round" => CUDAFunc::Func1("roundf"),
        "trunc" => CUDAFunc::Func1("truncf"),
        "fract" => CUDAFunc::Fract,
        "min" => CUDAFunc::Func2("fminf"),
        "max" => CUDAFunc::Func2("fmaxf"),
        "clamp" => CUDAFunc::Clamp,
        "saturate" => CUDAFunc::Saturate,

        // Interpolation
        "lerp" | "mix" => CUDAFunc::Lerp,
        "step" => CUDAFunc::Step,
        "smoothstep" => CUDAFunc::Smoothstep,

        // Other
        "copysign" => CUDAFunc::Func2("copysignf"),
        "fma" => CUDAFunc::Func3("fmaf"),
        "hypot" => CUDAFunc::Func2("hypotf"),

        _ => return None,
    })
}

/// Format a numeric literal for CUDA.
fn format_literal(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{:.1}f", n)
    } else {
        format!("{}f", n)
    }
}

/// Emit CUDA code for an expression AST.
pub fn emit_cuda(ast: &Ast) -> Result<CUDAExpr, CUDAError> {
    match ast {
        Ast::Num(n) => Ok(CUDAExpr::new(format_literal(*n))),

        Ast::Var(name) => Ok(CUDAExpr::new(name.clone())),

        Ast::BinOp(op, left, right) => {
            let l = emit_cuda(left)?;
            let r = emit_cuda(right)?;
            let code = match op {
                BinOp::Add => format!("({} + {})", l.code, r.code),
                BinOp::Sub => format!("({} - {})", l.code, r.code),
                BinOp::Mul => format!("({} * {})", l.code, r.code),
                BinOp::Div => format!("({} / {})", l.code, r.code),
                BinOp::Pow => format!("powf({}, {})", l.code, r.code),
                BinOp::Rem => format!("fmodf({}, {})", l.code, r.code),
                BinOp::BitAnd | BinOp::BitOr | BinOp::Shl | BinOp::Shr => {
                    return Err(CUDAError::UnsupportedFeature(
                        "bitwise ops on float".to_string(),
                    ));
                }
            };
            Ok(CUDAExpr::new(code))
        }

        Ast::UnaryOp(op, inner) => {
            let inner = emit_cuda(inner)?;
            let code = match op {
                UnaryOp::Neg => format!("(-{})", inner.code),
                UnaryOp::Not => {
                    let bool_expr = cond::scalar_to_bool(&inner.code);
                    cond::bool_to_scalar(&cond::emit_not(&bool_expr))
                }
                UnaryOp::BitNot => {
                    return Err(CUDAError::UnsupportedFeature(
                        "bitwise not on float".to_string(),
                    ));
                }
            };
            Ok(CUDAExpr::new(code))
        }

        Ast::Call(name, args) => {
            let arg_codes: Vec<String> = args
                .iter()
                .map(|a| emit_cuda(a).map(|e| e.code))
                .collect::<Result<_, _>>()?;

            let func =
                cuda_func_name(name).ok_or_else(|| CUDAError::UnknownFunction(name.clone()))?;

            let code = match func {
                CUDAFunc::Func1(f) => format!("{}({})", f, arg_codes[0]),
                CUDAFunc::Func2(f) => format!("{}({}, {})", f, arg_codes[0], arg_codes[1]),
                CUDAFunc::Func3(f) => {
                    format!(
                        "{}({}, {}, {})",
                        f, arg_codes[0], arg_codes[1], arg_codes[2]
                    )
                }
                CUDAFunc::Const(c) => c.to_string(),
                CUDAFunc::Tau => "(2.0f * M_PI)".to_string(),
                CUDAFunc::InverseSqrt => format!("rsqrtf({})", arg_codes[0]),
                CUDAFunc::Sign => format!("copysignf(1.0f, {})", arg_codes[0]),
                CUDAFunc::Fract => format!("({} - floorf({}))", arg_codes[0], arg_codes[0]),
                CUDAFunc::Clamp => format!(
                    "fminf(fmaxf({}, {}), {})",
                    arg_codes[0], arg_codes[1], arg_codes[2]
                ),
                CUDAFunc::Lerp => format!(
                    "({} + ({} - {}) * {})",
                    arg_codes[0], arg_codes[1], arg_codes[0], arg_codes[2]
                ),
                CUDAFunc::Step => format!("({} >= {} ? 1.0f : 0.0f)", arg_codes[1], arg_codes[0]),
                CUDAFunc::Smoothstep => format!(
                    "smoothstep_impl({}, {}, {})",
                    arg_codes[0], arg_codes[1], arg_codes[2]
                ),
                CUDAFunc::Saturate => format!("fminf(fmaxf({}, 0.0f), 1.0f)", arg_codes[0]),
            };
            Ok(CUDAExpr::new(code))
        }

        Ast::Compare(op, left, right) => {
            let l = emit_cuda(left)?;
            let r = emit_cuda(right)?;
            let bool_expr = cond::emit_compare(*op, &l.code, &r.code);
            Ok(CUDAExpr::new(cond::bool_to_scalar(&bool_expr)))
        }

        Ast::And(left, right) => {
            let l = emit_cuda(left)?;
            let r = emit_cuda(right)?;
            let l_bool = cond::scalar_to_bool(&l.code);
            let r_bool = cond::scalar_to_bool(&r.code);
            let bool_expr = cond::emit_and(&l_bool, &r_bool);
            Ok(CUDAExpr::new(cond::bool_to_scalar(&bool_expr)))
        }

        Ast::Or(left, right) => {
            let l = emit_cuda(left)?;
            let r = emit_cuda(right)?;
            let l_bool = cond::scalar_to_bool(&l.code);
            let r_bool = cond::scalar_to_bool(&r.code);
            let bool_expr = cond::emit_or(&l_bool, &r_bool);
            Ok(CUDAExpr::new(cond::bool_to_scalar(&bool_expr)))
        }

        Ast::If(cond_ast, then_ast, else_ast) => {
            let c = emit_cuda(cond_ast)?;
            let t = emit_cuda(then_ast)?;
            let e = emit_cuda(else_ast)?;
            let cond_bool = cond::scalar_to_bool(&c.code);
            Ok(CUDAExpr::new(cond::emit_if(&cond_bool, &t.code, &e.code)))
        }

        Ast::Let { name, value, body } => {
            // For simple emission, we inline the let binding
            // For proper function emission with statements, use emit_cuda_fn
            let val = emit_cuda(value)?;
            let body_code = emit_cuda(body)?.code;
            // Replace variable references in body - simple approach
            Ok(CUDAExpr::new(
                body_code.replace(name, &format!("({})", val.code)),
            ))
        }
    }
}

/// Emit a complete CUDA device function.
pub fn emit_cuda_fn(name: &str, ast: &Ast, params: &[&str]) -> Result<String, CUDAError> {
    let params_str = params
        .iter()
        .map(|n| format!("float {}", n))
        .collect::<Vec<_>>()
        .join(", ");

    let expr = emit_cuda(ast)?;

    Ok(format!(
        "__device__ float {}({}) {{\n    return {};\n}}",
        name, params_str, expr.code
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dew_core::Expr;

    fn emit(expr: &str) -> Result<CUDAExpr, CUDAError> {
        let expr = Expr::parse(expr).unwrap();
        emit_cuda(expr.ast())
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
    fn test_emit_cuda_fn() {
        let expr = Expr::parse("sin(x) + cos(y)").unwrap();
        let code = emit_cuda_fn("compute", expr.ast(), &["x", "y"]).unwrap();
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
