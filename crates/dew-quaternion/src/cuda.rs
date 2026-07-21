//! CUDA code generation for quaternion expressions.
//!
//! Uses CUDA's float4 for quaternions (x, y, z, w components).
//! float3 for Vec3. Quaternion operations require external functions.

use crate::Type;
use dew_cond::cuda as cond;
use dew_core::{Ast, BinOp, UnaryOp};
use std::collections::HashMap;

/// Error during CUDA code generation.
#[derive(Debug, Clone, PartialEq)]
pub enum CUDAError {
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

impl std::fmt::Display for CUDAError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CUDAError::UnknownVariable(name) => write!(f, "unknown variable: '{name}'"),
            CUDAError::UnknownFunction(name) => write!(f, "unknown function: '{name}'"),
            CUDAError::TypeMismatch { op, left, right } => {
                write!(f, "type mismatch for {op}: {left} vs {right}")
            }
            CUDAError::UnsupportedTypeForConditional(t) => {
                write!(f, "conditionals require scalar type, got {t}")
            }
            CUDAError::UnsupportedOperation(op) => write!(f, "unsupported operation: {op}"),
            CUDAError::UnsupportedFeature(feat) => {
                write!(f, "unsupported feature in CUDA codegen: {feat}")
            }
        }
    }
}

impl std::error::Error for CUDAError {}

/// Convert a Type to its CUDA representation.
pub fn type_to_cuda(t: Type) -> &'static str {
    match t {
        Type::Scalar => "float",
        Type::Vec3 => "float3",
        Type::Quaternion => "float4",
    }
}

/// Result of CUDA emission.
pub struct CUDAExpr {
    pub code: String,
    pub typ: Type,
}

/// Format a numeric literal for CUDA.
fn format_literal(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{:.1}f", n)
    } else {
        format!("{}f", n)
    }
}

/// Emit CUDA code for an AST with type propagation.
pub fn emit_cuda(ast: &Ast, var_types: &HashMap<String, Type>) -> Result<CUDAExpr, CUDAError> {
    match ast {
        Ast::Num(n) => Ok(CUDAExpr {
            code: format_literal(*n),
            typ: Type::Scalar,
        }),

        Ast::Var(name) => {
            let typ = var_types
                .get(name)
                .copied()
                .ok_or_else(|| CUDAError::UnknownVariable(name.clone()))?;
            Ok(CUDAExpr {
                code: name.clone(),
                typ,
            })
        }

        Ast::BinOp(op, left, right) => {
            let left_expr = emit_cuda(left, var_types)?;
            let right_expr = emit_cuda(right, var_types)?;
            emit_binop(*op, left_expr, right_expr)
        }

        Ast::UnaryOp(op, inner) => {
            let inner_expr = emit_cuda(inner, var_types)?;
            emit_unaryop(*op, inner_expr)
        }

        Ast::Call(name, args) => {
            let arg_exprs: Vec<CUDAExpr> = args
                .iter()
                .map(|a| emit_cuda(a, var_types))
                .collect::<Result<_, _>>()?;
            emit_function_call(name, arg_exprs)
        }

        Ast::Compare(op, left, right) => {
            let left_expr = emit_cuda(left, var_types)?;
            let right_expr = emit_cuda(right, var_types)?;
            if left_expr.typ != Type::Scalar || right_expr.typ != Type::Scalar {
                return Err(CUDAError::UnsupportedTypeForConditional(left_expr.typ));
            }
            let bool_expr = cond::emit_compare(*op, &left_expr.code, &right_expr.code);
            Ok(CUDAExpr {
                code: cond::bool_to_scalar(&bool_expr),
                typ: Type::Scalar,
            })
        }

        Ast::And(left, right) => {
            let left_expr = emit_cuda(left, var_types)?;
            let right_expr = emit_cuda(right, var_types)?;
            if left_expr.typ != Type::Scalar || right_expr.typ != Type::Scalar {
                return Err(CUDAError::UnsupportedTypeForConditional(left_expr.typ));
            }
            let l_bool = cond::scalar_to_bool(&left_expr.code);
            let r_bool = cond::scalar_to_bool(&right_expr.code);
            Ok(CUDAExpr {
                code: cond::bool_to_scalar(&cond::emit_and(&l_bool, &r_bool)),
                typ: Type::Scalar,
            })
        }

        Ast::Or(left, right) => {
            let left_expr = emit_cuda(left, var_types)?;
            let right_expr = emit_cuda(right, var_types)?;
            if left_expr.typ != Type::Scalar || right_expr.typ != Type::Scalar {
                return Err(CUDAError::UnsupportedTypeForConditional(left_expr.typ));
            }
            let l_bool = cond::scalar_to_bool(&left_expr.code);
            let r_bool = cond::scalar_to_bool(&right_expr.code);
            Ok(CUDAExpr {
                code: cond::bool_to_scalar(&cond::emit_or(&l_bool, &r_bool)),
                typ: Type::Scalar,
            })
        }

        Ast::If(cond_ast, then_ast, else_ast) => {
            let cond_expr = emit_cuda(cond_ast, var_types)?;
            let then_expr = emit_cuda(then_ast, var_types)?;
            let else_expr = emit_cuda(else_ast, var_types)?;
            if cond_expr.typ != Type::Scalar {
                return Err(CUDAError::UnsupportedTypeForConditional(cond_expr.typ));
            }
            let cond_bool = cond::scalar_to_bool(&cond_expr.code);
            Ok(CUDAExpr {
                code: cond::emit_if(&cond_bool, &then_expr.code, &else_expr.code),
                typ: then_expr.typ,
            })
        }

        Ast::Let { .. } => Err(CUDAError::UnsupportedFeature(
            "let bindings in expression context".to_string(),
        )),
    }
}

fn emit_binop(op: BinOp, left: CUDAExpr, right: CUDAExpr) -> Result<CUDAExpr, CUDAError> {
    match op {
        BinOp::Add | BinOp::Sub => {
            if left.typ != right.typ {
                return Err(CUDAError::TypeMismatch {
                    op: if op == BinOp::Add { "+" } else { "-" },
                    left: left.typ,
                    right: right.typ,
                });
            }
            let op_char = if op == BinOp::Add { '+' } else { '-' };
            Ok(CUDAExpr {
                code: format!("({} {} {})", left.code, op_char, right.code),
                typ: left.typ,
            })
        }
        BinOp::Mul => match (left.typ, right.typ) {
            (Type::Scalar, Type::Scalar) => Ok(CUDAExpr {
                code: format!("({} * {})", left.code, right.code),
                typ: Type::Scalar,
            }),
            (Type::Scalar, t) | (t, Type::Scalar) => Ok(CUDAExpr {
                code: format!("({} * {})", left.code, right.code),
                typ: t,
            }),
            (Type::Quaternion, Type::Quaternion) => Ok(CUDAExpr {
                code: format!("quat_mul({}, {})", left.code, right.code),
                typ: Type::Quaternion,
            }),
            (Type::Quaternion, Type::Vec3) => Ok(CUDAExpr {
                code: format!("quat_mul_vec3({}, {})", left.code, right.code),
                typ: Type::Vec3,
            }),
            _ => Err(CUDAError::TypeMismatch {
                op: "*",
                left: left.typ,
                right: right.typ,
            }),
        },
        BinOp::Div => match (left.typ, right.typ) {
            (Type::Scalar, Type::Scalar) => Ok(CUDAExpr {
                code: format!("({} / {})", left.code, right.code),
                typ: Type::Scalar,
            }),
            (t, Type::Scalar) => Ok(CUDAExpr {
                code: format!("({} / {})", left.code, right.code),
                typ: t,
            }),
            _ => Err(CUDAError::TypeMismatch {
                op: "/",
                left: left.typ,
                right: right.typ,
            }),
        },
        BinOp::Pow => {
            if left.typ == Type::Scalar && right.typ == Type::Scalar {
                Ok(CUDAExpr {
                    code: format!("powf({}, {})", left.code, right.code),
                    typ: Type::Scalar,
                })
            } else {
                Err(CUDAError::TypeMismatch {
                    op: "^",
                    left: left.typ,
                    right: right.typ,
                })
            }
        }
        _ => Err(CUDAError::UnsupportedOperation("bitwise")),
    }
}

fn emit_unaryop(op: UnaryOp, inner: CUDAExpr) -> Result<CUDAExpr, CUDAError> {
    match op {
        UnaryOp::Neg => Ok(CUDAExpr {
            code: format!("(-{})", inner.code),
            typ: inner.typ,
        }),
        UnaryOp::Not => {
            if inner.typ != Type::Scalar {
                return Err(CUDAError::UnsupportedTypeForConditional(inner.typ));
            }
            let bool_expr = cond::scalar_to_bool(&inner.code);
            Ok(CUDAExpr {
                code: cond::bool_to_scalar(&cond::emit_not(&bool_expr)),
                typ: Type::Scalar,
            })
        }
        UnaryOp::BitNot => Err(CUDAError::UnsupportedOperation("~")),
    }
}

fn emit_function_call(name: &str, args: Vec<CUDAExpr>) -> Result<CUDAExpr, CUDAError> {
    match name {
        "vec3" => {
            if args.len() != 3 || args.iter().any(|a| a.typ != Type::Scalar) {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            Ok(CUDAExpr {
                code: format!(
                    "make_float3({}, {}, {})",
                    args[0].code, args[1].code, args[2].code
                ),
                typ: Type::Vec3,
            })
        }
        "quat" => {
            if args.len() != 4 || args.iter().any(|a| a.typ != Type::Scalar) {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            Ok(CUDAExpr {
                code: format!(
                    "make_float4({}, {}, {}, {})",
                    args[0].code, args[1].code, args[2].code, args[3].code
                ),
                typ: Type::Quaternion,
            })
        }
        "conj" => {
            if args.len() != 1 || args[0].typ != Type::Quaternion {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            Ok(CUDAExpr {
                code: format!("quat_conj({})", args[0].code),
                typ: Type::Quaternion,
            })
        }
        "length" => {
            if args.len() != 1 {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(CUDAExpr {
                    code: format!("fabsf({})", args[0].code),
                    typ: Type::Scalar,
                }),
                Type::Vec3 => Ok(CUDAExpr {
                    code: format!("length3({})", args[0].code),
                    typ: Type::Scalar,
                }),
                Type::Quaternion => Ok(CUDAExpr {
                    code: format!("length4({})", args[0].code),
                    typ: Type::Scalar,
                }),
            }
        }
        "normalize" => {
            if args.len() != 1 {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Err(CUDAError::UnknownFunction(name.to_string())),
                Type::Vec3 => Ok(CUDAExpr {
                    code: format!("normalize3({})", args[0].code),
                    typ: Type::Vec3,
                }),
                Type::Quaternion => Ok(CUDAExpr {
                    code: format!("normalize4({})", args[0].code),
                    typ: Type::Quaternion,
                }),
            }
        }
        "inverse" => {
            if args.len() != 1 || args[0].typ != Type::Quaternion {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            Ok(CUDAExpr {
                code: format!("quat_inverse({})", args[0].code),
                typ: Type::Quaternion,
            })
        }
        "dot" => {
            if args.len() != 2 || args[0].typ != args[1].typ {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            let func = match args[0].typ {
                Type::Vec3 => "dot3",
                Type::Quaternion => "dot4",
                _ => return Err(CUDAError::UnknownFunction(name.to_string())),
            };
            Ok(CUDAExpr {
                code: format!("{}({}, {})", func, args[0].code, args[1].code),
                typ: Type::Scalar,
            })
        }
        "cross" => {
            if args.len() != 2 || args[0].typ != Type::Vec3 || args[1].typ != Type::Vec3 {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            Ok(CUDAExpr {
                code: format!("cross({}, {})", args[0].code, args[1].code),
                typ: Type::Vec3,
            })
        }
        "lerp" | "mix" => {
            if args.len() != 3 || args[2].typ != Type::Scalar {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(CUDAExpr {
                    code: format!("lerp({}, {}, {})", args[0].code, args[1].code, args[2].code),
                    typ: Type::Scalar,
                }),
                _ => Ok(CUDAExpr {
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
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            Ok(CUDAExpr {
                code: format!(
                    "quat_slerp({}, {}, {})",
                    args[0].code, args[1].code, args[2].code
                ),
                typ: Type::Quaternion,
            })
        }
        "axis_angle" => {
            if args.len() != 2 || args[0].typ != Type::Vec3 || args[1].typ != Type::Scalar {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            Ok(CUDAExpr {
                code: format!("quat_from_axis_angle({}, {})", args[0].code, args[1].code),
                typ: Type::Quaternion,
            })
        }
        "rotate" => {
            if args.len() != 2 || args[0].typ != Type::Vec3 || args[1].typ != Type::Quaternion {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            Ok(CUDAExpr {
                code: format!("quat_mul_vec3({}, {})", args[1].code, args[0].code),
                typ: Type::Vec3,
            })
        }
        _ => Err(CUDAError::UnknownFunction(name.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dew_core::Expr;

    fn emit(expr: &str, var_types: &[(&str, Type)]) -> Result<CUDAExpr, CUDAError> {
        let expr = Expr::parse(expr).unwrap();
        let types: HashMap<String, Type> =
            var_types.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        emit_cuda(expr.ast(), &types)
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
