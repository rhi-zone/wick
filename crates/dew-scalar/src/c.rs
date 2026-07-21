//! C code generation for scalar expressions.
//!
//! Compiles expression ASTs to C source code.
//! Uses math.h functions (sinf, cosf, etc.) and standard C float literals.

use dew_cond::c as cond;
use dew_core::{Ast, BinOp, UnaryOp};

/// C emission error.
#[derive(Debug, Clone, PartialEq)]
pub enum CError {
    /// Unknown function.
    UnknownFunction(String),
    /// Feature not supported in C codegen.
    UnsupportedFeature(String),
}

impl std::fmt::Display for CError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CError::UnknownFunction(name) => write!(f, "unknown function: '{name}'"),
            CError::UnsupportedFeature(feat) => {
                write!(f, "unsupported feature in C codegen: {feat}")
            }
        }
    }
}

impl std::error::Error for CError {}

/// Result of emitting C code.
#[derive(Debug, Clone)]
pub struct CExpr {
    /// The C expression string.
    pub code: String,
}

impl CExpr {
    pub fn new(code: impl Into<String>) -> Self {
        Self { code: code.into() }
    }
}

/// Returns the C equivalent for a scalar function name.
fn c_func_name(name: &str) -> Option<CFunc> {
    Some(match name {
        // Constants (from math.h)
        "pi" => CFunc::Const("M_PI"),
        "e" => CFunc::Const("M_E"),
        "tau" => CFunc::Tau, // M_PI * 2

        // Trig - standard math.h functions
        "sin" => CFunc::Func1("sinf"),
        "cos" => CFunc::Func1("cosf"),
        "tan" => CFunc::Func1("tanf"),
        "asin" => CFunc::Func1("asinf"),
        "acos" => CFunc::Func1("acosf"),
        "atan" => CFunc::Func1("atanf"),
        "atan2" => CFunc::Func2("atan2f"),
        "sinh" => CFunc::Func1("sinhf"),
        "cosh" => CFunc::Func1("coshf"),
        "tanh" => CFunc::Func1("tanhf"),

        // Exp/log
        "exp" => CFunc::Func1("expf"),
        "exp2" => CFunc::Func1("exp2f"),
        "log2" => CFunc::Func1("log2f"),
        "pow" => CFunc::Func2("powf"),
        "sqrt" => CFunc::Func1("sqrtf"),
        "log" | "ln" => CFunc::Func1("logf"),
        "log10" => CFunc::Func1("log10f"),
        "inversesqrt" => CFunc::InverseSqrt,

        // Common math
        "abs" => CFunc::Func1("fabsf"),
        "sign" => CFunc::Sign,
        "floor" => CFunc::Func1("floorf"),
        "ceil" => CFunc::Func1("ceilf"),
        "round" => CFunc::Func1("roundf"),
        "trunc" => CFunc::Func1("truncf"),
        "fract" => CFunc::Fract,
        "min" => CFunc::Func2("fminf"),
        "max" => CFunc::Func2("fmaxf"),
        "clamp" => CFunc::Clamp,
        "saturate" => CFunc::Saturate,

        // Interpolation
        "lerp" | "mix" => CFunc::Lerp,
        "step" => CFunc::Step,
        "smoothstep" => CFunc::Smoothstep,
        "inverse_lerp" => CFunc::InverseLerp,
        "remap" => CFunc::Remap,

        _ => return None,
    })
}

enum CFunc {
    /// Constant (e.g., M_PI)
    Const(&'static str),
    /// Single-argument function: func(arg)
    Func1(&'static str),
    /// Two-argument function: func(arg0, arg1)
    Func2(&'static str),
    /// tau = 2 * M_PI
    Tau,
    /// inversesqrt(x) -> 1.0f / sqrtf(x)
    InverseSqrt,
    /// sign(x) -> copysignf(1.0f, x) or comparison
    Sign,
    /// fract(x) -> x - floorf(x)
    Fract,
    /// clamp(x, lo, hi) -> fminf(fmaxf(x, lo), hi)
    Clamp,
    /// saturate(x) -> fminf(fmaxf(x, 0.0f), 1.0f)
    Saturate,
    /// lerp(a, b, t) -> a + (b - a) * t
    Lerp,
    /// step(edge, x) -> (x < edge) ? 0.0f : 1.0f
    Step,
    /// smoothstep(e0, e1, x)
    Smoothstep,
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

/// Emit C code for an AST.
pub fn emit_c(ast: &Ast) -> Result<CExpr, CError> {
    let emission = emit_full(ast)?;
    Ok(CExpr::new(emission.expr))
}

/// Generate a complete C function.
pub fn emit_c_fn(name: &str, ast: &Ast, params: &[&str]) -> Result<String, CError> {
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
fn emit_full(ast: &Ast) -> Result<Emission, CError> {
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
fn emit(ast: &Ast) -> Result<String, CError> {
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
                BinOp::Pow => Ok(format!("powf({}, {})", emit(left)?, emit(right)?)),
                BinOp::Rem => Ok(format!("fmodf({}, {})", emit(left)?, emit(right)?)),
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
            let func = c_func_name(name).ok_or_else(|| CError::UnknownFunction(name.clone()))?;

            let args_str: Vec<String> = args.iter().map(emit).collect::<Result<_, _>>()?;

            Ok(match func {
                CFunc::Const(path) => path.to_string(),
                CFunc::Tau => "(2.0f * M_PI)".to_string(),
                CFunc::Func1(func_name) => {
                    let arg = args_str.first().map(|s| s.as_str()).unwrap_or("0.0f");
                    format!("{}({})", func_name, arg)
                }
                CFunc::Func2(func_name) => {
                    let arg0 = args_str.first().map(|s| s.as_str()).unwrap_or("0.0f");
                    let arg1 = args_str.get(1).map(|s| s.as_str()).unwrap_or("0.0f");
                    format!("{}({}, {})", func_name, arg0, arg1)
                }
                CFunc::InverseSqrt => {
                    let arg = args_str.first().map(|s| s.as_str()).unwrap_or("1.0f");
                    format!("(1.0f / sqrtf({}))", arg)
                }
                CFunc::Sign => {
                    let arg = args_str.first().map(|s| s.as_str()).unwrap_or("0.0f");
                    // copysignf(1.0f, x) returns 1.0f with sign of x, but doesn't handle 0
                    // Use: (x > 0) - (x < 0) cast to float
                    format!("((float)(({arg} > 0.0f) - ({arg} < 0.0f)))")
                }
                CFunc::Fract => {
                    let arg = args_str.first().map(|s| s.as_str()).unwrap_or("0.0f");
                    format!("({arg} - floorf({arg}))")
                }
                CFunc::Clamp => {
                    let x = args_str.first().map(|s| s.as_str()).unwrap_or("0.0f");
                    let lo = args_str.get(1).map(|s| s.as_str()).unwrap_or("0.0f");
                    let hi = args_str.get(2).map(|s| s.as_str()).unwrap_or("1.0f");
                    format!("fminf(fmaxf({x}, {lo}), {hi})")
                }
                CFunc::Saturate => {
                    let arg = args_str.first().map(|s| s.as_str()).unwrap_or("0.0f");
                    format!("fminf(fmaxf({arg}, 0.0f), 1.0f)")
                }
                CFunc::Lerp => {
                    let a = args_str.first().map(|s| s.as_str()).unwrap_or("0.0f");
                    let b = args_str.get(1).map(|s| s.as_str()).unwrap_or("1.0f");
                    let t = args_str.get(2).map(|s| s.as_str()).unwrap_or("0.0f");
                    format!("({a} + ({b} - {a}) * {t})")
                }
                CFunc::Step => {
                    let edge = args_str.first().map(|s| s.as_str()).unwrap_or("0.0f");
                    let x = args_str.get(1).map(|s| s.as_str()).unwrap_or("0.0f");
                    format!("(({x} < {edge}) ? 0.0f : 1.0f)")
                }
                CFunc::Smoothstep => {
                    let e0 = args_str.first().map(|s| s.as_str()).unwrap_or("0.0f");
                    let e1 = args_str.get(1).map(|s| s.as_str()).unwrap_or("1.0f");
                    let x = args_str.get(2).map(|s| s.as_str()).unwrap_or("0.0f");
                    // In C we can't use block expressions, so expand inline
                    // t = clamp((x - e0) / (e1 - e0), 0, 1); t * t * (3 - 2*t)
                    // Use a compound statement approach that works in most C compilers
                    format!(
                        "(({{ float _t = fminf(fmaxf(({x} - {e0}) / ({e1} - {e0}), 0.0f), 1.0f); _t * _t * (3.0f - 2.0f * _t); }}))"
                    )
                }
                CFunc::InverseLerp => {
                    let a = args_str.first().map(|s| s.as_str()).unwrap_or("0.0f");
                    let b = args_str.get(1).map(|s| s.as_str()).unwrap_or("1.0f");
                    let v = args_str.get(2).map(|s| s.as_str()).unwrap_or("0.0f");
                    format!("(({v} - {a}) / ({b} - {a}))")
                }
                CFunc::Remap => {
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
                Err(CError::UnsupportedFeature(
                    "let in expression position (use emit_c_fn for full support)".to_string(),
                ))
            }
        }
    }
}

fn emit_with_parens(ast: &Ast, parent_op: Option<BinOp>, is_left: bool) -> Result<String, CError> {
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
        emit_c(expr.ast()).unwrap().code
    }

    #[test]
    fn test_constants() {
        assert_eq!(compile("pi()"), "M_PI");
        assert_eq!(compile("e()"), "M_E");
        assert_eq!(compile("tau()"), "(2.0f * M_PI)");
    }

    #[test]
    fn test_trig() {
        assert_eq!(compile("sin(x)"), "sinf(x)");
        assert_eq!(compile("cos(x)"), "cosf(x)");
        assert_eq!(compile("atan2(y, x)"), "atan2f(y, x)");
    }

    #[test]
    fn test_exp_log() {
        assert_eq!(compile("exp(x)"), "expf(x)");
        assert_eq!(compile("ln(x)"), "logf(x)");
        assert_eq!(compile("log10(x)"), "log10f(x)");
        assert_eq!(compile("pow(x, 2)"), "powf(x, 2.0f)");
        assert_eq!(compile("sqrt(x)"), "sqrtf(x)");
        assert_eq!(compile("inversesqrt(x)"), "(1.0f / sqrtf(x))");
    }

    #[test]
    fn test_common() {
        assert_eq!(compile("abs(x)"), "fabsf(x)");
        assert_eq!(compile("floor(x)"), "floorf(x)");
        assert_eq!(compile("clamp(x, 0, 1)"), "fminf(fmaxf(x, 0.0f), 1.0f)");
        assert_eq!(compile("saturate(x)"), "fminf(fmaxf(x, 0.0f), 1.0f)");
    }

    #[test]
    fn test_sign() {
        let code = compile("sign(x)");
        assert!(code.contains("float"));
        assert!(code.contains("> 0.0f"));
    }

    #[test]
    fn test_fract() {
        assert_eq!(compile("fract(x)"), "(x - floorf(x))");
    }

    #[test]
    fn test_interpolation() {
        assert_eq!(compile("lerp(a, b, t)"), "(a + (b - a) * t)");
        assert_eq!(compile("mix(a, b, t)"), "(a + (b - a) * t)");
        assert_eq!(compile("step(e, x)"), "((x < e) ? 0.0f : 1.0f)");
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
        assert_eq!(compile("x ^ 2"), "powf(x, 2.0f)");
    }

    #[test]
    fn test_c_fn() {
        let expr = Expr::parse("x + y").unwrap();
        let code = emit_c_fn("add", expr.ast(), &["x", "y"]).unwrap();
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
        let code = emit_c_fn("double_add", expr.ast(), &["x"]).unwrap();
        assert!(code.contains("float t = x * 2.0f;"));
        assert!(code.contains("return t + t;"));
    }

    #[test]
    fn test_nested_let() {
        let expr = Expr::parse("let a = x; let b = a * 2; b + 1").unwrap();
        let code = emit_c_fn("nested", expr.ast(), &["x"]).unwrap();
        assert!(code.contains("float a = x;"));
        assert!(code.contains("float b = a * 2.0f;"));
        assert!(code.contains("return b + 1.0f;"));
    }

    #[test]
    fn test_min_max() {
        assert_eq!(compile("min(a, b)"), "fminf(a, b)");
        assert_eq!(compile("max(a, b)"), "fmaxf(a, b)");
    }
}
