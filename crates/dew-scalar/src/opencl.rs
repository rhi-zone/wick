//! OpenCL code generation for scalar expressions.
//!
//! Compiles expression ASTs to OpenCL kernel code.
//! Uses OpenCL built-in math functions (sin, cos, etc. - no `f` suffix needed).

use dew_cond::opencl as cond;
use dew_core::{Ast, BinOp, UnaryOp};

/// OpenCL emission error.
#[derive(Debug, Clone, PartialEq)]
pub enum OpenCLError {
    /// Unknown function.
    UnknownFunction(String),
    /// Feature not supported in OpenCL codegen.
    UnsupportedFeature(String),
}

impl std::fmt::Display for OpenCLError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenCLError::UnknownFunction(name) => write!(f, "unknown function: '{name}'"),
            OpenCLError::UnsupportedFeature(feat) => {
                write!(f, "unsupported feature in OpenCL codegen: {feat}")
            }
        }
    }
}

impl std::error::Error for OpenCLError {}

/// Result of emitting OpenCL code.
#[derive(Debug, Clone)]
pub struct OpenCLExpr {
    /// The OpenCL expression string.
    pub code: String,
}

impl OpenCLExpr {
    pub fn new(code: impl Into<String>) -> Self {
        Self { code: code.into() }
    }
}

/// Returns the OpenCL equivalent for a scalar function name.
fn opencl_func_name(name: &str) -> Option<OpenCLFunc> {
    Some(match name {
        // Constants
        "pi" => OpenCLFunc::Const("M_PI_F"),
        "e" => OpenCLFunc::Const("M_E_F"),
        "tau" => OpenCLFunc::Tau, // M_PI_F * 2

        // Trig - OpenCL built-ins (no f suffix)
        "sin" => OpenCLFunc::Func1("sin"),
        "cos" => OpenCLFunc::Func1("cos"),
        "tan" => OpenCLFunc::Func1("tan"),
        "asin" => OpenCLFunc::Func1("asin"),
        "acos" => OpenCLFunc::Func1("acos"),
        "atan" => OpenCLFunc::Func1("atan"),
        "atan2" => OpenCLFunc::Func2("atan2"),
        "sinh" => OpenCLFunc::Func1("sinh"),
        "cosh" => OpenCLFunc::Func1("cosh"),
        "tanh" => OpenCLFunc::Func1("tanh"),

        // Exp/log
        "exp" => OpenCLFunc::Func1("exp"),
        "exp2" => OpenCLFunc::Func1("exp2"),
        "log2" => OpenCLFunc::Func1("log2"),
        "pow" => OpenCLFunc::Func2("pow"),
        "sqrt" => OpenCLFunc::Func1("sqrt"),
        "log" | "ln" => OpenCLFunc::Func1("log"),
        "log10" => OpenCLFunc::Func1("log10"),
        "inversesqrt" => OpenCLFunc::Func1("rsqrt"), // OpenCL has rsqrt built-in

        // Common math - OpenCL has these built-in
        "abs" => OpenCLFunc::Func1("fabs"),
        "sign" => OpenCLFunc::Func1("sign"), // OpenCL has sign built-in
        "floor" => OpenCLFunc::Func1("floor"),
        "ceil" => OpenCLFunc::Func1("ceil"),
        "round" => OpenCLFunc::Func1("round"),
        "trunc" => OpenCLFunc::Func1("trunc"),
        "fract" => OpenCLFunc::Fract, // x - floor(x)
        "min" => OpenCLFunc::Func2("fmin"),
        "max" => OpenCLFunc::Func2("fmax"),
        "clamp" => OpenCLFunc::Func3("clamp"), // OpenCL has clamp built-in
        "saturate" => OpenCLFunc::Saturate,

        // Interpolation
        "lerp" | "mix" => OpenCLFunc::Func3("mix"), // OpenCL uses mix
        "step" => OpenCLFunc::Func2Rev("step"),     // OpenCL step(edge, x)
        "smoothstep" => OpenCLFunc::Func3("smoothstep"), // OpenCL has smoothstep built-in
        "inverse_lerp" => OpenCLFunc::InverseLerp,
        "remap" => OpenCLFunc::Remap,

        // OpenCL-specific
        "mad" => OpenCLFunc::Func3("mad"), // fused multiply-add

        _ => return None,
    })
}

enum OpenCLFunc {
    /// Constant (e.g., M_PI_F)
    Const(&'static str),
    /// Single-argument function
    Func1(&'static str),
    /// Two-argument function
    Func2(&'static str),
    /// Two-argument function with reversed args (step)
    Func2Rev(&'static str),
    /// Three-argument function
    Func3(&'static str),
    /// tau = 2 * M_PI_F
    Tau,
    /// fract(x) -> x - floor(x)
    Fract,
    /// saturate(x) -> clamp(x, 0.0f, 1.0f)
    Saturate,
    /// inverse_lerp(a, b, v) -> (v - a) / (b - a)
    InverseLerp,
    /// remap(x, in_lo, in_hi, out_lo, out_hi)
    Remap,
}

/// Result of emitting code: accumulated statements + final expression.
struct Emission {
    statements: Vec<String>,
    expr: String,
}

impl Emission {
    fn expr_only(expr: String) -> Self {
        Self {
            statements: vec![],
            expr,
        }
    }

    fn with_statements(statements: Vec<String>, expr: String) -> Self {
        Self { statements, expr }
    }
}

/// Emit OpenCL code for an AST.
pub fn emit_opencl(ast: &Ast) -> Result<OpenCLExpr, OpenCLError> {
    let emission = emit_full(ast)?;
    Ok(OpenCLExpr::new(emission.expr))
}

/// Generate a complete OpenCL function.
pub fn emit_opencl_fn(name: &str, ast: &Ast, params: &[&str]) -> Result<String, OpenCLError> {
    let param_list: String = params
        .iter()
        .map(|p| format!("float {}", p))
        .collect::<Vec<_>>()
        .join(", ");

    let emission = emit_full(ast)?;

    let mut body = String::new();
    for stmt in &emission.statements {
        body.push_str("    ");
        body.push_str(stmt);
        body.push('\n');
    }
    body.push_str("    return ");
    body.push_str(&emission.expr);
    body.push(';');

    Ok(format!("float {}({}) {{\n{}\n}}", name, param_list, body))
}

/// Emit with full statement support.
fn emit_full(ast: &Ast) -> Result<Emission, OpenCLError> {
    match ast {
        Ast::Let { name, value, body } => {
            let value_emission = emit_full(value)?;
            let mut body_emission = emit_full(body)?;

            let mut statements = value_emission.statements;
            statements.push(format!("float {} = {};", name, value_emission.expr));
            statements.append(&mut body_emission.statements);

            Ok(Emission::with_statements(statements, body_emission.expr))
        }
        _ => Ok(Emission::expr_only(emit(ast)?)),
    }
}

/// Simple emit for expression-only nodes.
fn emit(ast: &Ast) -> Result<String, OpenCLError> {
    match ast {
        Ast::Num(n) => Ok(format_float(*n)),
        Ast::Var(name) => Ok(name.clone()),
        Ast::BinOp(op, left, right) => {
            let l = emit_with_parens(left, Some(*op), true)?;
            let r = emit_with_parens(right, Some(*op), false)?;
            match op {
                BinOp::Add => Ok(format!("{} + {}", l, r)),
                BinOp::Sub => Ok(format!("{} - {}", l, r)),
                BinOp::Mul => Ok(format!("{} * {}", l, r)),
                BinOp::Div => Ok(format!("{} / {}", l, r)),
                BinOp::Pow => Ok(format!("pow({}, {})", emit(left)?, emit(right)?)),
                BinOp::Rem => Ok(format!("fmod({}, {})", emit(left)?, emit(right)?)),
                BinOp::BitAnd => Ok(format!("((int){} & (int){})", l, r)),
                BinOp::BitOr => Ok(format!("((int){} | (int){})", l, r)),
                BinOp::Shl => Ok(format!("((int){} << (int){})", l, r)),
                BinOp::Shr => Ok(format!("((int){} >> (int){})", l, r)),
            }
        }
        Ast::UnaryOp(op, inner) => {
            let inner_str = emit_with_parens(inner, None, false)?;
            match op {
                UnaryOp::Neg => Ok(format!("-{}", inner_str)),
                UnaryOp::Not => {
                    let bool_expr = cond::scalar_to_bool(&inner_str);
                    Ok(cond::bool_to_scalar(&cond::emit_not(&bool_expr)))
                }
                UnaryOp::BitNot => Ok(format!("(~(int){})", inner_str)),
            }
        }
        Ast::Compare(op, left, right) => {
            let l = emit(left)?;
            let r = emit(right)?;
            let bool_expr = cond::emit_compare(*op, &l, &r);
            Ok(cond::bool_to_scalar(&bool_expr))
        }
        Ast::And(left, right) => {
            let l = emit(left)?;
            let r = emit(right)?;
            let l_bool = cond::scalar_to_bool(&l);
            let r_bool = cond::scalar_to_bool(&r);
            let bool_expr = cond::emit_and(&l_bool, &r_bool);
            Ok(cond::bool_to_scalar(&bool_expr))
        }
        Ast::Or(left, right) => {
            let l = emit(left)?;
            let r = emit(right)?;
            let l_bool = cond::scalar_to_bool(&l);
            let r_bool = cond::scalar_to_bool(&r);
            let bool_expr = cond::emit_or(&l_bool, &r_bool);
            Ok(cond::bool_to_scalar(&bool_expr))
        }
        Ast::If(cond_ast, then_ast, else_ast) => {
            let c = emit(cond_ast)?;
            let then_expr = emit(then_ast)?;
            let else_expr = emit(else_ast)?;
            let cond_bool = cond::scalar_to_bool(&c);
            Ok(cond::emit_if(&cond_bool, &then_expr, &else_expr))
        }
        Ast::Call(name, args) => {
            let func =
                opencl_func_name(name).ok_or_else(|| OpenCLError::UnknownFunction(name.clone()))?;

            let args_str: Vec<String> = args.iter().map(emit).collect::<Result<_, _>>()?;

            Ok(match func {
                OpenCLFunc::Const(path) => path.to_string(),
                OpenCLFunc::Tau => "(2.0f * M_PI_F)".to_string(),
                OpenCLFunc::Func1(func_name) => {
                    let arg = args_str.first().map(|s| s.as_str()).unwrap_or("0.0f");
                    format!("{}({})", func_name, arg)
                }
                OpenCLFunc::Func2(func_name) => {
                    let arg0 = args_str.first().map(|s| s.as_str()).unwrap_or("0.0f");
                    let arg1 = args_str.get(1).map(|s| s.as_str()).unwrap_or("0.0f");
                    format!("{}({}, {})", func_name, arg0, arg1)
                }
                OpenCLFunc::Func2Rev(func_name) => {
                    // step(edge, x) - OpenCL order matches our API
                    let arg0 = args_str.first().map(|s| s.as_str()).unwrap_or("0.0f");
                    let arg1 = args_str.get(1).map(|s| s.as_str()).unwrap_or("0.0f");
                    format!("{}({}, {})", func_name, arg0, arg1)
                }
                OpenCLFunc::Func3(func_name) => {
                    let arg0 = args_str.first().map(|s| s.as_str()).unwrap_or("0.0f");
                    let arg1 = args_str.get(1).map(|s| s.as_str()).unwrap_or("0.0f");
                    let arg2 = args_str.get(2).map(|s| s.as_str()).unwrap_or("0.0f");
                    format!("{}({}, {}, {})", func_name, arg0, arg1, arg2)
                }
                OpenCLFunc::Fract => {
                    let arg = args_str.first().map(|s| s.as_str()).unwrap_or("0.0f");
                    format!("({arg} - floor({arg}))")
                }
                OpenCLFunc::Saturate => {
                    let arg = args_str.first().map(|s| s.as_str()).unwrap_or("0.0f");
                    format!("clamp({arg}, 0.0f, 1.0f)")
                }
                OpenCLFunc::InverseLerp => {
                    let a = args_str.first().map(|s| s.as_str()).unwrap_or("0.0f");
                    let b = args_str.get(1).map(|s| s.as_str()).unwrap_or("1.0f");
                    let v = args_str.get(2).map(|s| s.as_str()).unwrap_or("0.0f");
                    format!("(({v} - {a}) / ({b} - {a}))")
                }
                OpenCLFunc::Remap => {
                    let x = args_str.first().map(|s| s.as_str()).unwrap_or("0.0f");
                    let in_lo = args_str.get(1).map(|s| s.as_str()).unwrap_or("0.0f");
                    let in_hi = args_str.get(2).map(|s| s.as_str()).unwrap_or("1.0f");
                    let out_lo = args_str.get(3).map(|s| s.as_str()).unwrap_or("0.0f");
                    let out_hi = args_str.get(4).map(|s| s.as_str()).unwrap_or("1.0f");
                    format!(
                        "({out_lo} + ({out_hi} - {out_lo}) * (({x} - {in_lo}) / ({in_hi} - {in_lo})))"
                    )
                }
            })
        }
        Ast::Let { .. } => {
            let emission = emit_full(ast)?;
            if emission.statements.is_empty() {
                Ok(emission.expr)
            } else {
                Err(OpenCLError::UnsupportedFeature(
                    "let in expression position (use emit_opencl_fn for full support)".to_string(),
                ))
            }
        }
    }
}

fn emit_with_parens(
    ast: &Ast,
    parent_op: Option<BinOp>,
    is_left: bool,
) -> Result<String, OpenCLError> {
    let inner = emit(ast)?;

    let needs_parens = match ast {
        Ast::BinOp(child_op, _, _) => {
            if let Some(parent) = parent_op {
                let parent_prec = precedence(parent);
                let child_prec = precedence(*child_op);
                if child_prec < parent_prec {
                    true
                } else if child_prec == parent_prec && !is_left {
                    matches!(parent, BinOp::Sub | BinOp::Div)
                } else {
                    false
                }
            } else {
                false
            }
        }
        _ => false,
    };

    if needs_parens {
        Ok(format!("({})", inner))
    } else {
        Ok(inner)
    }
}

fn precedence(op: BinOp) -> u8 {
    match op {
        BinOp::BitOr => 0,
        BinOp::BitAnd => 0,
        BinOp::Shl | BinOp::Shr => 0,
        BinOp::Add | BinOp::Sub => 1,
        BinOp::Mul | BinOp::Div | BinOp::Rem => 2,
        BinOp::Pow => 3,
    }
}

fn format_float(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{:.1}f", n)
    } else {
        format!("{}f", n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dew_core::Expr;

    fn compile(input: &str) -> String {
        let expr = Expr::parse(input).unwrap();
        emit_opencl(expr.ast()).unwrap().code
    }

    #[test]
    fn test_constants() {
        assert_eq!(compile("pi()"), "M_PI_F");
        assert_eq!(compile("e()"), "M_E_F");
        assert_eq!(compile("tau()"), "(2.0f * M_PI_F)");
    }

    #[test]
    fn test_trig() {
        assert_eq!(compile("sin(x)"), "sin(x)");
        assert_eq!(compile("cos(x)"), "cos(x)");
        assert_eq!(compile("atan2(y, x)"), "atan2(y, x)");
    }

    #[test]
    fn test_exp_log() {
        assert_eq!(compile("exp(x)"), "exp(x)");
        assert_eq!(compile("ln(x)"), "log(x)");
        assert_eq!(compile("log10(x)"), "log10(x)");
        assert_eq!(compile("pow(x, 2)"), "pow(x, 2.0f)");
        assert_eq!(compile("sqrt(x)"), "sqrt(x)");
        assert_eq!(compile("inversesqrt(x)"), "rsqrt(x)");
    }

    #[test]
    fn test_common() {
        assert_eq!(compile("abs(x)"), "fabs(x)");
        assert_eq!(compile("floor(x)"), "floor(x)");
        assert_eq!(compile("sign(x)"), "sign(x)");
        assert_eq!(compile("clamp(x, 0, 1)"), "clamp(x, 0.0f, 1.0f)");
        assert_eq!(compile("saturate(x)"), "clamp(x, 0.0f, 1.0f)");
    }

    #[test]
    fn test_fract() {
        assert_eq!(compile("fract(x)"), "(x - floor(x))");
    }

    #[test]
    fn test_interpolation() {
        assert_eq!(compile("lerp(a, b, t)"), "mix(a, b, t)");
        assert_eq!(compile("mix(a, b, t)"), "mix(a, b, t)");
        assert_eq!(compile("step(e, x)"), "step(e, x)");
        assert_eq!(compile("smoothstep(0, 1, x)"), "smoothstep(0.0f, 1.0f, x)");
    }

    #[test]
    fn test_inverse_lerp() {
        assert_eq!(
            compile("inverse_lerp(0, 10, x)"),
            "((x - 0.0f) / (10.0f - 0.0f))"
        );
    }

    #[test]
    fn test_operators() {
        assert_eq!(compile("x + y"), "x + y");
        assert_eq!(compile("x * y + z"), "x * y + z");
        assert_eq!(compile("(x + y) * z"), "(x + y) * z");
        assert_eq!(compile("-x"), "-x");
        assert_eq!(compile("x ^ 2"), "pow(x, 2.0f)");
    }

    #[test]
    fn test_opencl_fn() {
        let expr = Expr::parse("x + y").unwrap();
        let code = emit_opencl_fn("add", expr.ast(), &["x", "y"]).unwrap();
        assert!(code.contains("float add(float x, float y)"));
        assert!(code.contains("return x + y;"));
    }

    #[test]
    fn test_compare() {
        let code = compile("x < y");
        assert!(code.contains("?"));
        assert!(code.contains("(x < y)"));
    }

    #[test]
    fn test_if_then_else() {
        let code = compile("if x > 0 then 1 else 0");
        assert!(code.contains("?"));
        assert!(code.contains(":"));
    }

    #[test]
    fn test_and_or() {
        let and_code = compile("x > 0 and y > 0");
        assert!(and_code.contains("&&"));

        let or_code = compile("x < 0 or y < 0");
        assert!(or_code.contains("||"));
    }

    #[test]
    fn test_not() {
        let code = compile("not x");
        assert!(code.contains("!"));
    }

    #[test]
    fn test_let_in_fn() {
        let expr = Expr::parse("let t = x * 2; t + t").unwrap();
        let code = emit_opencl_fn("double_add", expr.ast(), &["x"]).unwrap();
        assert!(code.contains("float t = x * 2.0f;"));
        assert!(code.contains("return t + t;"));
    }

    #[test]
    fn test_min_max() {
        assert_eq!(compile("min(a, b)"), "fmin(a, b)");
        assert_eq!(compile("max(a, b)"), "fmax(a, b)");
    }

    #[test]
    fn test_mad() {
        assert_eq!(compile("mad(a, b, c)"), "mad(a, b, c)");
    }
}
