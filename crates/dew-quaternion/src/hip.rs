//! HIP code generation for quaternion expressions.
//!
//! Uses HIP's float4 for quaternions (x, y, z, w components).
//! float3 for Vec3. HIP is source-compatible with CUDA.

use crate::Type;
use dew_cond::hip as cond;
use dew_core::{Ast, BinOp, UnaryOp};
use std::collections::HashMap;

/// Error during HIP code generation.
#[derive(Debug, Clone, PartialEq)]
pub enum HIPError {
    UnknownVariable(String),
    UnknownFunction(String),
    TypeMismatch {
        op: &'static str,
        left: Type,
        right: Type,
    },
    UnsupportedTypeForConditional(Type),
    UnsupportedOperation(&'static str),
    UnsupportedFeature(String),
}

impl std::fmt::Display for HIPError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HIPError::UnknownVariable(name) => write!(f, "unknown variable: '{name}'"),
            HIPError::UnknownFunction(name) => write!(f, "unknown function: '{name}'"),
            HIPError::TypeMismatch { op, left, right } => {
                write!(f, "type mismatch for {op}: {left} vs {right}")
            }
            HIPError::UnsupportedTypeForConditional(t) => {
                write!(f, "conditionals require scalar type, got {t}")
            }
            HIPError::UnsupportedOperation(op) => write!(f, "unsupported operation: {op}"),
            HIPError::UnsupportedFeature(feat) => {
                write!(f, "unsupported feature in HIP codegen: {feat}")
            }
        }
    }
}

impl std::error::Error for HIPError {}

/// Convert a Type to its HIP representation.
pub fn type_to_hip(t: Type) -> &'static str {
    match t {
        Type::Scalar => "float",
        Type::Vec3 => "float3",
        Type::Quaternion => "float4",
    }
}

/// Result of HIP emission.
pub struct HIPExpr {
    pub code: String,
    pub typ: Type,
}

/// Format a numeric literal for HIP.
fn format_literal(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{:.1}f", n)
    } else {
        format!("{}f", n)
    }
}

/// Emit HIP code for an AST with type propagation.
pub fn emit_hip(ast: &Ast, var_types: &HashMap<String, Type>) -> Result<HIPExpr, HIPError> {
    match ast {
        Ast::Num(n) => Ok(HIPExpr {
            code: format_literal(*n),
            typ: Type::Scalar,
        }),

        Ast::Var(name) => {
            let typ = var_types
                .get(name)
                .copied()
                .ok_or_else(|| HIPError::UnknownVariable(name.clone()))?;
            Ok(HIPExpr {
                code: name.clone(),
                typ,
            })
        }

        Ast::BinOp(op, left, right) => {
            let left_expr = emit_hip(left, var_types)?;
            let right_expr = emit_hip(right, var_types)?;
            emit_binop(*op, left_expr, right_expr)
        }

        Ast::UnaryOp(op, inner) => {
            let inner_expr = emit_hip(inner, var_types)?;
            emit_unaryop(*op, inner_expr)
        }

        Ast::Call(name, args) => {
            let arg_exprs: Vec<HIPExpr> = args
                .iter()
                .map(|a| emit_hip(a, var_types))
                .collect::<Result<_, _>>()?;
            emit_function_call(name, arg_exprs)
        }

        Ast::Compare(op, left, right) => {
            let left_expr = emit_hip(left, var_types)?;
            let right_expr = emit_hip(right, var_types)?;
            if left_expr.typ != Type::Scalar || right_expr.typ != Type::Scalar {
                return Err(HIPError::UnsupportedTypeForConditional(left_expr.typ));
            }
            let bool_expr = cond::emit_compare(*op, &left_expr.code, &right_expr.code);
            Ok(HIPExpr {
                code: cond::bool_to_scalar(&bool_expr),
                typ: Type::Scalar,
            })
        }

        Ast::And(left, right) => {
            let left_expr = emit_hip(left, var_types)?;
            let right_expr = emit_hip(right, var_types)?;
            if left_expr.typ != Type::Scalar || right_expr.typ != Type::Scalar {
                return Err(HIPError::UnsupportedTypeForConditional(left_expr.typ));
            }
            let l_bool = cond::scalar_to_bool(&left_expr.code);
            let r_bool = cond::scalar_to_bool(&right_expr.code);
            Ok(HIPExpr {
                code: cond::bool_to_scalar(&cond::emit_and(&l_bool, &r_bool)),
                typ: Type::Scalar,
            })
        }

        Ast::Or(left, right) => {
            let left_expr = emit_hip(left, var_types)?;
            let right_expr = emit_hip(right, var_types)?;
            if left_expr.typ != Type::Scalar || right_expr.typ != Type::Scalar {
                return Err(HIPError::UnsupportedTypeForConditional(left_expr.typ));
            }
            let l_bool = cond::scalar_to_bool(&left_expr.code);
            let r_bool = cond::scalar_to_bool(&right_expr.code);
            Ok(HIPExpr {
                code: cond::bool_to_scalar(&cond::emit_or(&l_bool, &r_bool)),
                typ: Type::Scalar,
            })
        }

        Ast::If(cond_ast, then_ast, else_ast) => {
            let cond_expr = emit_hip(cond_ast, var_types)?;
            let then_expr = emit_hip(then_ast, var_types)?;
            let else_expr = emit_hip(else_ast, var_types)?;
            if cond_expr.typ != Type::Scalar {
                return Err(HIPError::UnsupportedTypeForConditional(cond_expr.typ));
            }
            let cond_bool = cond::scalar_to_bool(&cond_expr.code);
            Ok(HIPExpr {
                code: cond::emit_if(&cond_bool, &then_expr.code, &else_expr.code),
                typ: then_expr.typ,
            })
        }

        Ast::Let { .. } => Err(HIPError::UnsupportedFeature(
            "let bindings in expression context".to_string(),
        )),
    }
}

fn emit_binop(op: BinOp, left: HIPExpr, right: HIPExpr) -> Result<HIPExpr, HIPError> {
    match op {
        BinOp::Add | BinOp::Sub => {
            if left.typ != right.typ {
                return Err(HIPError::TypeMismatch {
                    op: if op == BinOp::Add { "+" } else { "-" },
                    left: left.typ,
                    right: right.typ,
                });
            }
            let op_char = if op == BinOp::Add { '+' } else { '-' };
            Ok(HIPExpr {
                code: format!("({} {} {})", left.code, op_char, right.code),
                typ: left.typ,
            })
        }
        BinOp::Mul => match (left.typ, right.typ) {
            (Type::Scalar, Type::Scalar) => Ok(HIPExpr {
                code: format!("({} * {})", left.code, right.code),
                typ: Type::Scalar,
            }),
            (Type::Scalar, t) | (t, Type::Scalar) => Ok(HIPExpr {
                code: format!("({} * {})", left.code, right.code),
                typ: t,
            }),
            (Type::Quaternion, Type::Quaternion) => Ok(HIPExpr {
                code: format!("quat_mul({}, {})", left.code, right.code),
                typ: Type::Quaternion,
            }),
            (Type::Quaternion, Type::Vec3) => Ok(HIPExpr {
                code: format!("quat_mul_vec3({}, {})", left.code, right.code),
                typ: Type::Vec3,
            }),
            _ => Err(HIPError::TypeMismatch {
                op: "*",
                left: left.typ,
                right: right.typ,
            }),
        },
        BinOp::Div => match (left.typ, right.typ) {
            (Type::Scalar, Type::Scalar) => Ok(HIPExpr {
                code: format!("({} / {})", left.code, right.code),
                typ: Type::Scalar,
            }),
            (t, Type::Scalar) => Ok(HIPExpr {
                code: format!("({} / {})", left.code, right.code),
                typ: t,
            }),
            _ => Err(HIPError::TypeMismatch {
                op: "/",
                left: left.typ,
                right: right.typ,
            }),
        },
        BinOp::Pow => {
            if left.typ == Type::Scalar && right.typ == Type::Scalar {
                Ok(HIPExpr {
                    code: format!("powf({}, {})", left.code, right.code),
                    typ: Type::Scalar,
                })
            } else {
                Err(HIPError::TypeMismatch {
                    op: "^",
                    left: left.typ,
                    right: right.typ,
                })
            }
        }
        _ => Err(HIPError::UnsupportedOperation("bitwise")),
    }
}

fn emit_unaryop(op: UnaryOp, inner: HIPExpr) -> Result<HIPExpr, HIPError> {
    match op {
        UnaryOp::Neg => Ok(HIPExpr {
            code: format!("(-{})", inner.code),
            typ: inner.typ,
        }),
        UnaryOp::Not => {
            if inner.typ != Type::Scalar {
                return Err(HIPError::UnsupportedTypeForConditional(inner.typ));
            }
            let bool_expr = cond::scalar_to_bool(&inner.code);
            Ok(HIPExpr {
                code: cond::bool_to_scalar(&cond::emit_not(&bool_expr)),
                typ: Type::Scalar,
            })
        }
        UnaryOp::BitNot => Err(HIPError::UnsupportedOperation("~")),
    }
}

fn emit_function_call(name: &str, args: Vec<HIPExpr>) -> Result<HIPExpr, HIPError> {
    match name {
        "vec3" => {
            if args.len() != 3 || args.iter().any(|a| a.typ != Type::Scalar) {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            Ok(HIPExpr {
                code: format!(
                    "make_float3({}, {}, {})",
                    args[0].code, args[1].code, args[2].code
                ),
                typ: Type::Vec3,
            })
        }
        "quat" => {
            if args.len() != 4 || args.iter().any(|a| a.typ != Type::Scalar) {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            Ok(HIPExpr {
                code: format!(
                    "make_float4({}, {}, {}, {})",
                    args[0].code, args[1].code, args[2].code, args[3].code
                ),
                typ: Type::Quaternion,
            })
        }
        "conj" => {
            if args.len() != 1 || args[0].typ != Type::Quaternion {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            Ok(HIPExpr {
                code: format!("quat_conj({})", args[0].code),
                typ: Type::Quaternion,
            })
        }
        "length" => {
            if args.len() != 1 {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(HIPExpr {
                    code: format!("fabsf({})", args[0].code),
                    typ: Type::Scalar,
                }),
                Type::Vec3 => Ok(HIPExpr {
                    code: format!("length3({})", args[0].code),
                    typ: Type::Scalar,
                }),
                Type::Quaternion => Ok(HIPExpr {
                    code: format!("length4({})", args[0].code),
                    typ: Type::Scalar,
                }),
            }
        }
        "normalize" => {
            if args.len() != 1 {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Err(HIPError::UnknownFunction(name.to_string())),
                Type::Vec3 => Ok(HIPExpr {
                    code: format!("normalize3({})", args[0].code),
                    typ: Type::Vec3,
                }),
                Type::Quaternion => Ok(HIPExpr {
                    code: format!("normalize4({})", args[0].code),
                    typ: Type::Quaternion,
                }),
            }
        }
        "inverse" => {
            if args.len() != 1 || args[0].typ != Type::Quaternion {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            Ok(HIPExpr {
                code: format!("quat_inverse({})", args[0].code),
                typ: Type::Quaternion,
            })
        }
        "dot" => {
            if args.len() != 2 || args[0].typ != args[1].typ {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            let func = match args[0].typ {
                Type::Vec3 => "dot3",
                Type::Quaternion => "dot4",
                _ => return Err(HIPError::UnknownFunction(name.to_string())),
            };
            Ok(HIPExpr {
                code: format!("{}({}, {})", func, args[0].code, args[1].code),
                typ: Type::Scalar,
            })
        }
        "cross" => {
            if args.len() != 2 || args[0].typ != Type::Vec3 || args[1].typ != Type::Vec3 {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            Ok(HIPExpr {
                code: format!("cross({}, {})", args[0].code, args[1].code),
                typ: Type::Vec3,
            })
        }
        "lerp" | "mix" => {
            if args.len() != 3 || args[2].typ != Type::Scalar {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(HIPExpr {
                    code: format!("lerp({}, {}, {})", args[0].code, args[1].code, args[2].code),
                    typ: Type::Scalar,
                }),
                _ => Ok(HIPExpr {
                    code: format!(
                        "({} + ({} - {}) * {})",
                        args[0].code, args[1].code, args[0].code, args[2].code
                    ),
                    typ: args[0].typ,
                }),
            }
        }
        "slerp" => {
            if args.len() != 3
                || args[0].typ != Type::Quaternion
                || args[1].typ != Type::Quaternion
                || args[2].typ != Type::Scalar
            {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            Ok(HIPExpr {
                code: format!(
                    "quat_slerp({}, {}, {})",
                    args[0].code, args[1].code, args[2].code
                ),
                typ: Type::Quaternion,
            })
        }
        "axis_angle" => {
            if args.len() != 2 || args[0].typ != Type::Vec3 || args[1].typ != Type::Scalar {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            Ok(HIPExpr {
                code: format!("quat_from_axis_angle({}, {})", args[0].code, args[1].code),
                typ: Type::Quaternion,
            })
        }
        "rotate" => {
            if args.len() != 2 || args[0].typ != Type::Vec3 || args[1].typ != Type::Quaternion {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            Ok(HIPExpr {
                code: format!("quat_mul_vec3({}, {})", args[1].code, args[0].code),
                typ: Type::Vec3,
            })
        }
        _ => Err(HIPError::UnknownFunction(name.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dew_core::Expr;

    fn emit(expr: &str, var_types: &[(&str, Type)]) -> Result<HIPExpr, HIPError> {
        let expr = Expr::parse(expr).unwrap();
        let types: HashMap<String, Type> =
            var_types.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        emit_hip(expr.ast(), &types)
    }

    #[test]
    fn test_quaternion_add() {
        let result = emit("a + b", &[("a", Type::Quaternion), ("b", Type::Quaternion)]).unwrap();
        assert_eq!(result.typ, Type::Quaternion);
        assert_eq!(result.code, "(a + b)");
    }

    #[test]
    fn test_quaternion_mul() {
        let result = emit("a * b", &[("a", Type::Quaternion), ("b", Type::Quaternion)]).unwrap();
        assert!(result.code.contains("quat_mul"));
    }

    #[test]
    fn test_quat_constructor() {
        let result = emit(
            "quat(x, y, z, w)",
            &[
                ("x", Type::Scalar),
                ("y", Type::Scalar),
                ("z", Type::Scalar),
                ("w", Type::Scalar),
            ],
        )
        .unwrap();
        assert!(result.code.contains("make_float4"));
    }

    #[test]
    fn test_normalize() {
        let result = emit("normalize(q)", &[("q", Type::Quaternion)]).unwrap();
        assert!(result.code.contains("normalize4"));
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
        assert!(result.code.contains("quat_slerp"));
    }
}
