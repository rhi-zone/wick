//! C code generation for complex expressions.
//!
//! Complex numbers are represented using a struct: typedef struct { float re, im; } complex_t;
//! Uses function-based operations (e.g., complex_add, complex_mul, complex_conj).

use crate::Type;
use dew_cond::c as cond;
use dew_core::{Ast, BinOp, UnaryOp};
use std::collections::HashMap;

/// Error during C code generation.
#[derive(Debug, Clone, PartialEq)]
pub enum CError {
    UnknownVariable(String),
    UnknownFunction(String),
    TypeMismatch {
        op: &'static str,
        left: Type,
        right: Type,
    },
    /// Conditionals require scalar types.
    UnsupportedTypeForConditional(Type),
    /// Operation not supported for this type.
    UnsupportedOperation(&'static str),
    /// Feature not supported in expression-only codegen.
    UnsupportedFeature(String),
}

impl std::fmt::Display for CError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CError::UnknownVariable(name) => write!(f, "unknown variable: '{name}'"),
            CError::UnknownFunction(name) => write!(f, "unknown function: '{name}'"),
            CError::TypeMismatch { op, left, right } => {
                write!(f, "type mismatch for {op}: {left} vs {right}")
            }
            CError::UnsupportedTypeForConditional(t) => {
                write!(f, "conditionals require scalar type, got {t}")
            }
            CError::UnsupportedOperation(op) => {
                write!(f, "unsupported operation for complex: {op}")
            }
            CError::UnsupportedFeature(feat) => {
                write!(f, "unsupported feature in C codegen: {feat}")
            }
        }
    }
}

impl std::error::Error for CError {}

/// Convert a Type to its C representation.
pub fn type_to_c(t: Type) -> &'static str {
    match t {
        Type::Scalar => "float",
        Type::Complex => "complex_t",
    }
}

/// Result of C emission: code string and its type.
pub struct CExpr {
    pub code: String,
    pub typ: Type,
}

/// Result of emitting code with statement support.
struct Emission {
    statements: Vec<String>,
    expr: String,
    typ: Type,
}

impl Emission {
    fn expr_only(expr: String, typ: Type) -> Self {
        Self {
            statements: vec![],
            expr,
            typ,
        }
    }

    fn with_statements(statements: Vec<String>, expr: String, typ: Type) -> Self {
        Self {
            statements,
            expr,
            typ,
        }
    }
}

/// Format a numeric literal for C.
fn format_literal(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{:.1}f", n)
    } else {
        format!("{}f", n)
    }
}

/// Emit C code for an AST with type propagation.
pub fn emit_c(ast: &Ast, var_types: &HashMap<String, Type>) -> Result<CExpr, CError> {
    match ast {
        Ast::Num(n) => Ok(CExpr {
            code: format_literal(*n),
            typ: Type::Scalar,
        }),

        Ast::Var(name) => {
            let typ = var_types
                .get(name)
                .copied()
                .ok_or_else(|| CError::UnknownVariable(name.clone()))?;
            Ok(CExpr {
                code: name.clone(),
                typ,
            })
        }

        Ast::BinOp(op, left, right) => {
            let left_expr = emit_c(left, var_types)?;
            let right_expr = emit_c(right, var_types)?;
            emit_binop(*op, left_expr, right_expr)
        }

        Ast::UnaryOp(op, inner) => {
            let inner_expr = emit_c(inner, var_types)?;
            emit_unaryop(*op, inner_expr)
        }

        Ast::Compare(op, left, right) => {
            let left_expr = emit_c(left, var_types)?;
            let right_expr = emit_c(right, var_types)?;
            if left_expr.typ != Type::Scalar || right_expr.typ != Type::Scalar {
                return Err(CError::UnsupportedTypeForConditional(Type::Complex));
            }
            let bool_expr = cond::emit_compare(*op, &left_expr.code, &right_expr.code);
            Ok(CExpr {
                code: cond::bool_to_scalar(&bool_expr),
                typ: Type::Scalar,
            })
        }

        Ast::And(left, right) => {
            let left_expr = emit_c(left, var_types)?;
            let right_expr = emit_c(right, var_types)?;
            if left_expr.typ != Type::Scalar || right_expr.typ != Type::Scalar {
                return Err(CError::UnsupportedTypeForConditional(Type::Complex));
            }
            let l_bool = cond::scalar_to_bool(&left_expr.code);
            let r_bool = cond::scalar_to_bool(&right_expr.code);
            let bool_expr = cond::emit_and(&l_bool, &r_bool);
            Ok(CExpr {
                code: cond::bool_to_scalar(&bool_expr),
                typ: Type::Scalar,
            })
        }

        Ast::Or(left, right) => {
            let left_expr = emit_c(left, var_types)?;
            let right_expr = emit_c(right, var_types)?;
            if left_expr.typ != Type::Scalar || right_expr.typ != Type::Scalar {
                return Err(CError::UnsupportedTypeForConditional(Type::Complex));
            }
            let l_bool = cond::scalar_to_bool(&left_expr.code);
            let r_bool = cond::scalar_to_bool(&right_expr.code);
            let bool_expr = cond::emit_or(&l_bool, &r_bool);
            Ok(CExpr {
                code: cond::bool_to_scalar(&bool_expr),
                typ: Type::Scalar,
            })
        }

        Ast::If(cond_ast, then_ast, else_ast) => {
            let cond_expr = emit_c(cond_ast, var_types)?;
            let then_expr = emit_c(then_ast, var_types)?;
            let else_expr = emit_c(else_ast, var_types)?;
            if cond_expr.typ != Type::Scalar {
                return Err(CError::UnsupportedTypeForConditional(cond_expr.typ));
            }
            if then_expr.typ != else_expr.typ {
                return Err(CError::TypeMismatch {
                    op: "if/else",
                    left: then_expr.typ,
                    right: else_expr.typ,
                });
            }
            let cond_bool = cond::scalar_to_bool(&cond_expr.code);
            Ok(CExpr {
                code: cond::emit_if(&cond_bool, &then_expr.code, &else_expr.code),
                typ: then_expr.typ,
            })
        }

        Ast::Call(name, args) => {
            let arg_exprs: Vec<CExpr> = args
                .iter()
                .map(|a| emit_c(a, var_types))
                .collect::<Result<_, _>>()?;
            emit_function_call(name, arg_exprs)
        }

        Ast::Let { .. } => {
            let emission = emit_full(ast, var_types)?;
            if emission.statements.is_empty() {
                Ok(CExpr {
                    code: emission.expr,
                    typ: emission.typ,
                })
            } else {
                Err(CError::UnsupportedFeature(
                    "let in expression position (use emit_c_fn for full support)".to_string(),
                ))
            }
        }
    }
}

/// Emit a complete C function with let statement support.
pub fn emit_c_fn(
    name: &str,
    ast: &Ast,
    params: &[(&str, Type)],
    return_type: Type,
) -> Result<String, CError> {
    let var_types: HashMap<String, Type> =
        params.iter().map(|(n, t)| (n.to_string(), *t)).collect();
    let emission = emit_full(ast, &var_types)?;

    let param_list: Vec<String> = params
        .iter()
        .map(|(n, t)| format!("{} {}", type_to_c(*t), n))
        .collect();

    let mut body = String::new();
    for stmt in emission.statements {
        body.push_str("    ");
        body.push_str(&stmt);
        body.push('\n');
    }
    body.push_str("    return ");
    body.push_str(&emission.expr);
    body.push(';');

    Ok(format!(
        "{} {}({}) {{\n{}\n}}",
        type_to_c(return_type),
        name,
        param_list.join(", "),
        body
    ))
}

/// Emit with full statement support for let bindings.
fn emit_full(ast: &Ast, var_types: &HashMap<String, Type>) -> Result<Emission, CError> {
    match ast {
        Ast::Let { name, value, body } => {
            let value_emission = emit_full(value, var_types)?;
            let mut new_var_types = var_types.clone();
            new_var_types.insert(name.clone(), value_emission.typ);
            let mut body_emission = emit_full(body, &new_var_types)?;

            let mut statements = value_emission.statements;
            statements.push(format!(
                "{} {} = {};",
                type_to_c(value_emission.typ),
                name,
                value_emission.expr
            ));
            statements.append(&mut body_emission.statements);

            Ok(Emission::with_statements(
                statements,
                body_emission.expr,
                body_emission.typ,
            ))
        }
        _ => {
            let result = emit_c(ast, var_types)?;
            Ok(Emission::expr_only(result.code, result.typ))
        }
    }
}

fn emit_binop(op: BinOp, left: CExpr, right: CExpr) -> Result<CExpr, CError> {
    match op {
        BinOp::Add => emit_add(left, right),
        BinOp::Sub => emit_sub(left, right),
        BinOp::Mul => emit_mul(left, right),
        BinOp::Div => emit_div(left, right),
        BinOp::Pow => emit_pow(left, right),
        BinOp::Rem => Err(CError::UnsupportedOperation("%")),
        BinOp::BitAnd => Err(CError::UnsupportedOperation("&")),
        BinOp::BitOr => Err(CError::UnsupportedOperation("|")),
        BinOp::Shl => Err(CError::UnsupportedOperation("<<")),
        BinOp::Shr => Err(CError::UnsupportedOperation(">>")),
    }
}

fn emit_add(left: CExpr, right: CExpr) -> Result<CExpr, CError> {
    match (left.typ, right.typ) {
        (Type::Scalar, Type::Scalar) => Ok(CExpr {
            code: format!("({} + {})", left.code, right.code),
            typ: Type::Scalar,
        }),
        (Type::Complex, Type::Complex) => Ok(CExpr {
            code: format!("complex_add({}, {})", left.code, right.code),
            typ: Type::Complex,
        }),
        (Type::Scalar, Type::Complex) => Ok(CExpr {
            code: format!(
                "complex_add(complex_new({}, 0.0f), {})",
                left.code, right.code
            ),
            typ: Type::Complex,
        }),
        (Type::Complex, Type::Scalar) => Ok(CExpr {
            code: format!(
                "complex_add({}, complex_new({}, 0.0f))",
                left.code, right.code
            ),
            typ: Type::Complex,
        }),
    }
}

fn emit_sub(left: CExpr, right: CExpr) -> Result<CExpr, CError> {
    match (left.typ, right.typ) {
        (Type::Scalar, Type::Scalar) => Ok(CExpr {
            code: format!("({} - {})", left.code, right.code),
            typ: Type::Scalar,
        }),
        (Type::Complex, Type::Complex) => Ok(CExpr {
            code: format!("complex_sub({}, {})", left.code, right.code),
            typ: Type::Complex,
        }),
        (Type::Scalar, Type::Complex) => Ok(CExpr {
            code: format!(
                "complex_sub(complex_new({}, 0.0f), {})",
                left.code, right.code
            ),
            typ: Type::Complex,
        }),
        (Type::Complex, Type::Scalar) => Ok(CExpr {
            code: format!(
                "complex_sub({}, complex_new({}, 0.0f))",
                left.code, right.code
            ),
            typ: Type::Complex,
        }),
    }
}

fn emit_mul(left: CExpr, right: CExpr) -> Result<CExpr, CError> {
    match (left.typ, right.typ) {
        (Type::Scalar, Type::Scalar) => Ok(CExpr {
            code: format!("({} * {})", left.code, right.code),
            typ: Type::Scalar,
        }),
        (Type::Scalar, Type::Complex) => Ok(CExpr {
            code: format!("complex_scale({}, {})", right.code, left.code),
            typ: Type::Complex,
        }),
        (Type::Complex, Type::Scalar) => Ok(CExpr {
            code: format!("complex_scale({}, {})", left.code, right.code),
            typ: Type::Complex,
        }),
        (Type::Complex, Type::Complex) => Ok(CExpr {
            code: format!("complex_mul({}, {})", left.code, right.code),
            typ: Type::Complex,
        }),
    }
}

fn emit_div(left: CExpr, right: CExpr) -> Result<CExpr, CError> {
    match (left.typ, right.typ) {
        (Type::Scalar, Type::Scalar) => Ok(CExpr {
            code: format!("({} / {})", left.code, right.code),
            typ: Type::Scalar,
        }),
        (Type::Complex, Type::Scalar) => Ok(CExpr {
            code: format!("complex_scale({}, 1.0f / {})", left.code, right.code),
            typ: Type::Complex,
        }),
        (Type::Complex, Type::Complex) => Ok(CExpr {
            code: format!("complex_div({}, {})", left.code, right.code),
            typ: Type::Complex,
        }),
        _ => Err(CError::TypeMismatch {
            op: "/",
            left: left.typ,
            right: right.typ,
        }),
    }
}

fn emit_pow(base: CExpr, exp: CExpr) -> Result<CExpr, CError> {
    match (base.typ, exp.typ) {
        (Type::Scalar, Type::Scalar) => Ok(CExpr {
            code: format!("powf({}, {})", base.code, exp.code),
            typ: Type::Scalar,
        }),
        (Type::Complex, Type::Scalar) => Ok(CExpr {
            code: format!("complex_powf({}, {})", base.code, exp.code),
            typ: Type::Complex,
        }),
        (Type::Complex, Type::Complex) => Ok(CExpr {
            code: format!("complex_pow({}, {})", base.code, exp.code),
            typ: Type::Complex,
        }),
        _ => Err(CError::TypeMismatch {
            op: "^",
            left: base.typ,
            right: exp.typ,
        }),
    }
}

fn emit_unaryop(op: UnaryOp, inner: CExpr) -> Result<CExpr, CError> {
    match op {
        UnaryOp::Neg => {
            let code = match inner.typ {
                Type::Scalar => format!("(-{})", inner.code),
                Type::Complex => format!("complex_neg({})", inner.code),
            };
            Ok(CExpr {
                code,
                typ: inner.typ,
            })
        }
        UnaryOp::Not => {
            if inner.typ != Type::Scalar {
                return Err(CError::UnsupportedTypeForConditional(inner.typ));
            }
            let bool_expr = cond::scalar_to_bool(&inner.code);
            Ok(CExpr {
                code: cond::bool_to_scalar(&cond::emit_not(&bool_expr)),
                typ: Type::Scalar,
            })
        }
        UnaryOp::BitNot => Err(CError::UnsupportedOperation("~")),
    }
}

fn emit_function_call(name: &str, args: Vec<CExpr>) -> Result<CExpr, CError> {
    match name {
        "re" => {
            if args.len() != 1 || args[0].typ != Type::Complex {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!("{}.re", args[0].code),
                typ: Type::Scalar,
            })
        }

        "im" => {
            if args.len() != 1 || args[0].typ != Type::Complex {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!("{}.im", args[0].code),
                typ: Type::Scalar,
            })
        }

        "conj" => {
            if args.len() != 1 || args[0].typ != Type::Complex {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!("complex_conj({})", args[0].code),
                typ: Type::Complex,
            })
        }

        "abs" => {
            if args.len() != 1 {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(CExpr {
                    code: format!("fabsf({})", args[0].code),
                    typ: Type::Scalar,
                }),
                Type::Complex => Ok(CExpr {
                    code: format!("complex_abs({})", args[0].code),
                    typ: Type::Scalar,
                }),
            }
        }

        "arg" => {
            if args.len() != 1 || args[0].typ != Type::Complex {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!("complex_arg({})", args[0].code),
                typ: Type::Scalar,
            })
        }

        "norm" => {
            if args.len() != 1 || args[0].typ != Type::Complex {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!("complex_norm({})", args[0].code),
                typ: Type::Scalar,
            })
        }

        "exp" => {
            if args.len() != 1 {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(CExpr {
                    code: format!("expf({})", args[0].code),
                    typ: Type::Scalar,
                }),
                Type::Complex => Ok(CExpr {
                    code: format!("complex_exp({})", args[0].code),
                    typ: Type::Complex,
                }),
            }
        }

        "log" => {
            if args.len() != 1 {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(CExpr {
                    code: format!("logf({})", args[0].code),
                    typ: Type::Scalar,
                }),
                Type::Complex => Ok(CExpr {
                    code: format!("complex_log({})", args[0].code),
                    typ: Type::Complex,
                }),
            }
        }

        "sqrt" => {
            if args.len() != 1 {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(CExpr {
                    code: format!("sqrtf({})", args[0].code),
                    typ: Type::Scalar,
                }),
                Type::Complex => Ok(CExpr {
                    code: format!("complex_sqrt({})", args[0].code),
                    typ: Type::Complex,
                }),
            }
        }

        "polar" => {
            if args.len() != 2 || args[0].typ != Type::Scalar || args[1].typ != Type::Scalar {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!("complex_from_polar({}, {})", args[0].code, args[1].code),
                typ: Type::Complex,
            })
        }

        "complex" => {
            if args.len() != 2 || args[0].typ != Type::Scalar || args[1].typ != Type::Scalar {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!("complex_new({}, {})", args[0].code, args[1].code),
                typ: Type::Complex,
            })
        }

        _ => Err(CError::UnknownFunction(name.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dew_core::Expr;

    fn emit(expr: &str, var_types: &[(&str, Type)]) -> Result<CExpr, CError> {
        let expr = Expr::parse(expr).unwrap();
        let types: HashMap<String, Type> =
            var_types.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        emit_c(expr.ast(), &types)
    }

    #[test]
    fn test_complex_add() {
        let result = emit("a + b", &[("a", Type::Complex), ("b", Type::Complex)]).unwrap();
        assert_eq!(result.typ, Type::Complex);
        assert!(result.code.contains("complex_add"));
    }

    #[test]
    fn test_complex_mul() {
        let result = emit("a * b", &[("a", Type::Complex), ("b", Type::Complex)]).unwrap();
        assert_eq!(result.typ, Type::Complex);
        assert!(result.code.contains("complex_mul"));
    }

    #[test]
    fn test_re() {
        let result = emit("re(z)", &[("z", Type::Complex)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert!(result.code.contains(".re"));
    }

    #[test]
    fn test_im() {
        let result = emit("im(z)", &[("z", Type::Complex)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert!(result.code.contains(".im"));
    }

    #[test]
    fn test_abs() {
        let result = emit("abs(z)", &[("z", Type::Complex)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert!(result.code.contains("complex_abs"));
    }

    #[test]
    fn test_conj() {
        let result = emit("conj(z)", &[("z", Type::Complex)]).unwrap();
        assert_eq!(result.typ, Type::Complex);
        assert!(result.code.contains("complex_conj"));
    }

    #[test]
    fn test_exp() {
        let result = emit("exp(z)", &[("z", Type::Complex)]).unwrap();
        assert_eq!(result.typ, Type::Complex);
        assert!(result.code.contains("complex_exp"));
    }

    #[test]
    fn test_let_in_fn() {
        let expr = Expr::parse("let w = z * z; w + z").unwrap();
        let code = emit_c_fn(
            "square_add",
            expr.ast(),
            &[("z", Type::Complex)],
            Type::Complex,
        )
        .unwrap();
        assert!(code.contains("complex_t w ="));
        assert!(code.contains("complex_add(w, z)"));
    }

    #[test]
    fn test_polar() {
        let result = emit(
            "polar(r, theta)",
            &[("r", Type::Scalar), ("theta", Type::Scalar)],
        )
        .unwrap();
        assert_eq!(result.typ, Type::Complex);
        assert!(result.code.contains("complex_from_polar"));
    }
}
