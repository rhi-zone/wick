//! C code generation for quaternion expressions.
//!
//! Quaternions use a struct: typedef struct { float x, y, z, w; } quat_t;
//! Vectors use a struct: typedef struct { float x, y, z; } vec3_t;
//! Uses function-based operations (e.g., quat_mul, quat_conj, quat_normalize).

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
                write!(f, "unsupported operation for quaternion: {op}")
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
        Type::Vec3 => "vec3_t",
        Type::Quaternion => "quat_t",
    }
}

/// Result of C emission: code string and its type.
pub struct CExpr {
    pub code: String,
    pub typ: Type,
}

/// Result of full C emission with accumulated statements.
struct Emission {
    statements: Vec<String>,
    expr: String,
    typ: Type,
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

        Ast::Call(name, args) => {
            let arg_exprs: Vec<CExpr> = args
                .iter()
                .map(|a| emit_c(a, var_types))
                .collect::<Result<_, _>>()?;
            emit_function_call(name, arg_exprs)
        }

        Ast::Compare(op, left, right) => {
            let left_expr = emit_c(left, var_types)?;
            let right_expr = emit_c(right, var_types)?;
            if left_expr.typ != Type::Scalar || right_expr.typ != Type::Scalar {
                return Err(CError::UnsupportedTypeForConditional(left_expr.typ));
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
                return Err(CError::UnsupportedTypeForConditional(left_expr.typ));
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
                return Err(CError::UnsupportedTypeForConditional(left_expr.typ));
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

        Ast::Let { .. } => {
            let emission = emit_full(ast, var_types)?;
            if !emission.statements.is_empty() {
                return Err(CError::UnsupportedFeature(
                    "let bindings in expression context (use emit_c_fn)".to_string(),
                ));
            }
            Ok(CExpr {
                code: emission.expr,
                typ: emission.typ,
            })
        }
    }
}

/// Emit C with full statement support.
fn emit_full(ast: &Ast, var_types: &HashMap<String, Type>) -> Result<Emission, CError> {
    match ast {
        Ast::Let { name, value, body } => {
            let value_emission = emit_full(value, var_types)?;
            let mut new_var_types = var_types.clone();
            new_var_types.insert(name.clone(), value_emission.typ);
            let body_emission = emit_full(body, &new_var_types)?;

            let mut statements = value_emission.statements;
            statements.push(format!(
                "{} {} = {};",
                type_to_c(value_emission.typ),
                name,
                value_emission.expr
            ));
            statements.extend(body_emission.statements);

            Ok(Emission {
                statements,
                expr: body_emission.expr,
                typ: body_emission.typ,
            })
        }
        _ => {
            let result = emit_c(ast, var_types)?;
            Ok(Emission {
                statements: vec![],
                expr: result.code,
                typ: result.typ,
            })
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

    let params_str = params
        .iter()
        .map(|(n, t)| format!("{} {}", type_to_c(*t), n))
        .collect::<Vec<_>>()
        .join(", ");

    let mut body = String::new();
    for stmt in &emission.statements {
        body.push_str("    ");
        body.push_str(stmt);
        body.push('\n');
    }
    body.push_str("    return ");
    body.push_str(&emission.expr);
    body.push(';');

    Ok(format!(
        "{} {}({}) {{\n{}\n}}",
        type_to_c(return_type),
        name,
        params_str,
        body
    ))
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
        (Type::Vec3, Type::Vec3) => Ok(CExpr {
            code: format!("vec3_add({}, {})", left.code, right.code),
            typ: Type::Vec3,
        }),
        (Type::Quaternion, Type::Quaternion) => Ok(CExpr {
            code: format!("quat_add({}, {})", left.code, right.code),
            typ: Type::Quaternion,
        }),
        _ => Err(CError::TypeMismatch {
            op: "+",
            left: left.typ,
            right: right.typ,
        }),
    }
}

fn emit_sub(left: CExpr, right: CExpr) -> Result<CExpr, CError> {
    match (left.typ, right.typ) {
        (Type::Scalar, Type::Scalar) => Ok(CExpr {
            code: format!("({} - {})", left.code, right.code),
            typ: Type::Scalar,
        }),
        (Type::Vec3, Type::Vec3) => Ok(CExpr {
            code: format!("vec3_sub({}, {})", left.code, right.code),
            typ: Type::Vec3,
        }),
        (Type::Quaternion, Type::Quaternion) => Ok(CExpr {
            code: format!("quat_sub({}, {})", left.code, right.code),
            typ: Type::Quaternion,
        }),
        _ => Err(CError::TypeMismatch {
            op: "-",
            left: left.typ,
            right: right.typ,
        }),
    }
}

fn emit_mul(left: CExpr, right: CExpr) -> Result<CExpr, CError> {
    match (left.typ, right.typ) {
        (Type::Scalar, Type::Scalar) => Ok(CExpr {
            code: format!("({} * {})", left.code, right.code),
            typ: Type::Scalar,
        }),

        (Type::Scalar, Type::Vec3) | (Type::Vec3, Type::Scalar) => {
            let (scalar, vec) = if left.typ == Type::Scalar {
                (&left.code, &right.code)
            } else {
                (&right.code, &left.code)
            };
            Ok(CExpr {
                code: format!("vec3_scale({}, {})", vec, scalar),
                typ: Type::Vec3,
            })
        }

        (Type::Scalar, Type::Quaternion) | (Type::Quaternion, Type::Scalar) => {
            let (scalar, quat) = if left.typ == Type::Scalar {
                (&left.code, &right.code)
            } else {
                (&right.code, &left.code)
            };
            Ok(CExpr {
                code: format!("quat_scale({}, {})", quat, scalar),
                typ: Type::Quaternion,
            })
        }

        // Quaternion * Quaternion (Hamilton product)
        (Type::Quaternion, Type::Quaternion) => Ok(CExpr {
            code: format!("quat_mul({}, {})", left.code, right.code),
            typ: Type::Quaternion,
        }),

        // Quaternion * Vec3 (rotate vector)
        (Type::Quaternion, Type::Vec3) => Ok(CExpr {
            code: format!("quat_mul_vec3({}, {})", left.code, right.code),
            typ: Type::Vec3,
        }),

        _ => Err(CError::TypeMismatch {
            op: "*",
            left: left.typ,
            right: right.typ,
        }),
    }
}

fn emit_div(left: CExpr, right: CExpr) -> Result<CExpr, CError> {
    match (left.typ, right.typ) {
        (Type::Scalar, Type::Scalar) => Ok(CExpr {
            code: format!("({} / {})", left.code, right.code),
            typ: Type::Scalar,
        }),
        (Type::Vec3, Type::Scalar) => Ok(CExpr {
            code: format!("vec3_scale({}, 1.0f / {})", left.code, right.code),
            typ: Type::Vec3,
        }),
        (Type::Quaternion, Type::Scalar) => Ok(CExpr {
            code: format!("quat_scale({}, 1.0f / {})", left.code, right.code),
            typ: Type::Quaternion,
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
                Type::Vec3 => format!("vec3_neg({})", inner.code),
                Type::Quaternion => format!("quat_neg({})", inner.code),
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
        "conj" => {
            if args.len() != 1 || args[0].typ != Type::Quaternion {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!("quat_conj({})", args[0].code),
                typ: Type::Quaternion,
            })
        }

        "length" => {
            if args.len() != 1 {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            let func = match args[0].typ {
                Type::Scalar => {
                    return Ok(CExpr {
                        code: format!("fabsf({})", args[0].code),
                        typ: Type::Scalar,
                    });
                }
                Type::Vec3 => "vec3_length",
                Type::Quaternion => "quat_length",
            };
            Ok(CExpr {
                code: format!("{}({})", func, args[0].code),
                typ: Type::Scalar,
            })
        }

        "normalize" => {
            if args.len() != 1 {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            let func = match args[0].typ {
                Type::Scalar => return Err(CError::UnknownFunction(name.to_string())),
                Type::Vec3 => "vec3_normalize",
                Type::Quaternion => "quat_normalize",
            };
            Ok(CExpr {
                code: format!("{}({})", func, args[0].code),
                typ: args[0].typ,
            })
        }

        "inverse" => {
            if args.len() != 1 || args[0].typ != Type::Quaternion {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!("quat_inverse({})", args[0].code),
                typ: Type::Quaternion,
            })
        }

        "dot" => {
            if args.len() != 2 || args[0].typ != args[1].typ {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            let func = match args[0].typ {
                Type::Scalar => return Err(CError::UnknownFunction(name.to_string())),
                Type::Vec3 => "vec3_dot",
                Type::Quaternion => "quat_dot",
            };
            Ok(CExpr {
                code: format!("{}({}, {})", func, args[0].code, args[1].code),
                typ: Type::Scalar,
            })
        }

        "lerp" => {
            if args.len() != 3 || args[2].typ != Type::Scalar {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            let func = match args[0].typ {
                Type::Scalar => {
                    return Ok(CExpr {
                        code: format!(
                            "({} + ({} - {}) * {})",
                            args[0].code, args[1].code, args[0].code, args[2].code
                        ),
                        typ: Type::Scalar,
                    });
                }
                Type::Vec3 => "vec3_lerp",
                Type::Quaternion => "quat_lerp",
            };
            Ok(CExpr {
                code: format!(
                    "{}({}, {}, {})",
                    func, args[0].code, args[1].code, args[2].code
                ),
                typ: args[0].typ,
            })
        }

        "slerp" => {
            if args.len() != 3
                || args[0].typ != Type::Quaternion
                || args[1].typ != Type::Quaternion
                || args[2].typ != Type::Scalar
            {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!(
                    "quat_slerp({}, {}, {})",
                    args[0].code, args[1].code, args[2].code
                ),
                typ: Type::Quaternion,
            })
        }

        "axis_angle" => {
            if args.len() != 2 || args[0].typ != Type::Vec3 || args[1].typ != Type::Scalar {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!("quat_from_axis_angle({}, {})", args[0].code, args[1].code),
                typ: Type::Quaternion,
            })
        }

        "rotate" => {
            if args.len() != 2 || args[0].typ != Type::Vec3 || args[1].typ != Type::Quaternion {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!("quat_mul_vec3({}, {})", args[1].code, args[0].code),
                typ: Type::Vec3,
            })
        }

        "cross" => {
            if args.len() != 2 || args[0].typ != Type::Vec3 || args[1].typ != Type::Vec3 {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!("vec3_cross({}, {})", args[0].code, args[1].code),
                typ: Type::Vec3,
            })
        }

        "vec3" => {
            if args.len() != 3 || args.iter().any(|a| a.typ != Type::Scalar) {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!(
                    "vec3_new({}, {}, {})",
                    args[0].code, args[1].code, args[2].code
                ),
                typ: Type::Vec3,
            })
        }

        "quat" => {
            if args.len() != 4 || args.iter().any(|a| a.typ != Type::Scalar) {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!(
                    "quat_new({}, {}, {}, {})",
                    args[0].code, args[1].code, args[2].code, args[3].code
                ),
                typ: Type::Quaternion,
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
    fn test_quaternion_add() {
        let result = emit("a + b", &[("a", Type::Quaternion), ("b", Type::Quaternion)]).unwrap();
        assert_eq!(result.typ, Type::Quaternion);
        assert!(result.code.contains("quat_add"));
    }

    #[test]
    fn test_quaternion_mul() {
        let result = emit("a * b", &[("a", Type::Quaternion), ("b", Type::Quaternion)]).unwrap();
        assert_eq!(result.typ, Type::Quaternion);
        assert!(result.code.contains("quat_mul"));
    }

    #[test]
    fn test_quaternion_rotate_vec() {
        let result = emit("q * v", &[("q", Type::Quaternion), ("v", Type::Vec3)]).unwrap();
        assert_eq!(result.typ, Type::Vec3);
        assert!(result.code.contains("quat_mul_vec3"));
    }

    #[test]
    fn test_normalize() {
        let result = emit("normalize(q)", &[("q", Type::Quaternion)]).unwrap();
        assert_eq!(result.typ, Type::Quaternion);
        assert!(result.code.contains("quat_normalize"));
    }

    #[test]
    fn test_conj() {
        let result = emit("conj(q)", &[("q", Type::Quaternion)]).unwrap();
        assert_eq!(result.typ, Type::Quaternion);
        assert!(result.code.contains("quat_conj"));
    }

    #[test]
    fn test_dot() {
        let result = emit(
            "dot(a, b)",
            &[("a", Type::Quaternion), ("b", Type::Quaternion)],
        )
        .unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert!(result.code.contains("quat_dot"));
    }

    #[test]
    fn test_axis_angle() {
        let result = emit(
            "axis_angle(v, a)",
            &[("v", Type::Vec3), ("a", Type::Scalar)],
        )
        .unwrap();
        assert_eq!(result.typ, Type::Quaternion);
        assert!(result.code.contains("quat_from_axis_angle"));
    }

    #[test]
    fn test_rotate() {
        let result = emit(
            "rotate(v, q)",
            &[("v", Type::Vec3), ("q", Type::Quaternion)],
        )
        .unwrap();
        assert_eq!(result.typ, Type::Vec3);
        assert!(result.code.contains("quat_mul_vec3"));
    }

    #[test]
    fn test_slerp() {
        let result = emit(
            "slerp(a, b, t)",
            &[
                ("a", Type::Quaternion),
                ("b", Type::Quaternion),
                ("t", Type::Scalar),
            ],
        )
        .unwrap();
        assert_eq!(result.typ, Type::Quaternion);
        assert!(result.code.contains("quat_slerp"));
    }

    #[test]
    fn test_emit_c_fn_simple() {
        let expr = Expr::parse("normalize(q)").unwrap();
        let code = emit_c_fn(
            "norm_quat",
            expr.ast(),
            &[("q", Type::Quaternion)],
            Type::Quaternion,
        )
        .unwrap();
        assert!(code.contains("quat_t norm_quat(quat_t q)"));
        assert!(code.contains("quat_normalize"));
    }

    #[test]
    fn test_emit_c_fn_with_let() {
        let expr = Expr::parse("let sq = q * q; sq + q").unwrap();
        let code = emit_c_fn(
            "square_add",
            expr.ast(),
            &[("q", Type::Quaternion)],
            Type::Quaternion,
        )
        .unwrap();
        assert!(code.contains("quat_t sq ="));
        assert!(code.contains("quat_add(sq, q)"));
    }
}
