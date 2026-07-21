//! CUDA code generation for complex number expressions.
//!
//! Uses CUDA's float2 for complex numbers (.x = real, .y = imag).
//! Basic arithmetic operators work via CUDA's helper_math.h when included.

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
        Type::Complex => "float2",
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
            let bool_expr = cond::emit_and(&l_bool, &r_bool);
            Ok(CUDAExpr {
                code: cond::bool_to_scalar(&bool_expr),
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
            let bool_expr = cond::emit_or(&l_bool, &r_bool);
            Ok(CUDAExpr {
                code: cond::bool_to_scalar(&bool_expr),
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
        BinOp::Add => {
            let typ = if left.typ == Type::Complex || right.typ == Type::Complex {
                Type::Complex
            } else {
                Type::Scalar
            };
            Ok(CUDAExpr {
                code: format!("({} + {})", left.code, right.code),
                typ,
            })
        }
        BinOp::Sub => {
            let typ = if left.typ == Type::Complex || right.typ == Type::Complex {
                Type::Complex
            } else {
                Type::Scalar
            };
            Ok(CUDAExpr {
                code: format!("({} - {})", left.code, right.code),
                typ,
            })
        }
        BinOp::Mul => match (left.typ, right.typ) {
            (Type::Scalar, Type::Scalar) => Ok(CUDAExpr {
                code: format!("({} * {})", left.code, right.code),
                typ: Type::Scalar,
            }),
            (Type::Scalar, Type::Complex) | (Type::Complex, Type::Scalar) => Ok(CUDAExpr {
                code: format!("({} * {})", left.code, right.code),
                typ: Type::Complex,
            }),
            (Type::Complex, Type::Complex) => Ok(CUDAExpr {
                code: format!("complex_mul({}, {})", left.code, right.code),
                typ: Type::Complex,
            }),
        },
        BinOp::Div => match (left.typ, right.typ) {
            (Type::Scalar, Type::Scalar) => Ok(CUDAExpr {
                code: format!("({} / {})", left.code, right.code),
                typ: Type::Scalar,
            }),
            (Type::Complex, Type::Scalar) => Ok(CUDAExpr {
                code: format!("({} / {})", left.code, right.code),
                typ: Type::Complex,
            }),
            (Type::Complex, Type::Complex) => Ok(CUDAExpr {
                code: format!("complex_div({}, {})", left.code, right.code),
                typ: Type::Complex,
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
                Ok(CUDAExpr {
                    code: format!("complex_pow({}, {})", left.code, right.code),
                    typ: Type::Complex,
                })
            }
        }
        _ => Err(CUDAError::TypeMismatch {
            op: "unsupported",
            left: left.typ,
            right: right.typ,
        }),
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
        UnaryOp::BitNot => Err(CUDAError::TypeMismatch {
            op: "~",
            left: inner.typ,
            right: inner.typ,
        }),
    }
}

fn emit_function_call(name: &str, args: Vec<CUDAExpr>) -> Result<CUDAExpr, CUDAError> {
    match name {
        "complex" => {
            if args.len() != 2 {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            Ok(CUDAExpr {
                code: format!("make_float2({}, {})", args[0].code, args[1].code),
                typ: Type::Complex,
            })
        }
        "polar" => {
            if args.len() != 2 {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            // polar(r, theta) = r * (cos(theta) + i*sin(theta))
            Ok(CUDAExpr {
                code: format!(
                    "make_float2({} * cosf({}), {} * sinf({}))",
                    args[0].code, args[1].code, args[0].code, args[1].code
                ),
                typ: Type::Complex,
            })
        }
        "re" => {
            if args.len() != 1 || args[0].typ != Type::Complex {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            Ok(CUDAExpr {
                code: format!("{}.x", args[0].code),
                typ: Type::Scalar,
            })
        }
        "im" => {
            if args.len() != 1 || args[0].typ != Type::Complex {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            Ok(CUDAExpr {
                code: format!("{}.y", args[0].code),
                typ: Type::Scalar,
            })
        }
        "conj" => {
            if args.len() != 1 || args[0].typ != Type::Complex {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            Ok(CUDAExpr {
                code: format!("make_float2({}.x, -{}.y)", args[0].code, args[0].code),
                typ: Type::Complex,
            })
        }
        "abs" => {
            if args.len() != 1 {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(CUDAExpr {
                    code: format!("fabsf({})", args[0].code),
                    typ: Type::Scalar,
                }),
                Type::Complex => Ok(CUDAExpr {
                    code: format!(
                        "sqrtf({}.x * {}.x + {}.y * {}.y)",
                        args[0].code, args[0].code, args[0].code, args[0].code
                    ),
                    typ: Type::Scalar,
                }),
            }
        }
        "arg" => {
            if args.len() != 1 || args[0].typ != Type::Complex {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            Ok(CUDAExpr {
                code: format!("atan2f({}.y, {}.x)", args[0].code, args[0].code),
                typ: Type::Scalar,
            })
        }
        "norm" => {
            if args.len() != 1 || args[0].typ != Type::Complex {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            Ok(CUDAExpr {
                code: format!(
                    "({}.x * {}.x + {}.y * {}.y)",
                    args[0].code, args[0].code, args[0].code, args[0].code
                ),
                typ: Type::Scalar,
            })
        }
        "exp" => {
            if args.len() != 1 {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(CUDAExpr {
                    code: format!("expf({})", args[0].code),
                    typ: Type::Scalar,
                }),
                Type::Complex => Ok(CUDAExpr {
                    code: format!("complex_exp({})", args[0].code),
                    typ: Type::Complex,
                }),
            }
        }
        "log" => {
            if args.len() != 1 {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(CUDAExpr {
                    code: format!("logf({})", args[0].code),
                    typ: Type::Scalar,
                }),
                Type::Complex => Ok(CUDAExpr {
                    code: format!("complex_log({})", args[0].code),
                    typ: Type::Complex,
                }),
            }
        }
        "sqrt" => {
            if args.len() != 1 {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(CUDAExpr {
                    code: format!("sqrtf({})", args[0].code),
                    typ: Type::Scalar,
                }),
                Type::Complex => Ok(CUDAExpr {
                    code: format!("complex_sqrt({})", args[0].code),
                    typ: Type::Complex,
                }),
            }
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
    fn test_complex_add() {
        let result = emit("a + b", &[("a", Type::Complex), ("b", Type::Complex)]).unwrap();
        assert_eq!(result.typ, Type::Complex);
    }

    #[test]
    fn test_complex_mul() {
        let result = emit("a * b", &[("a", Type::Complex), ("b", Type::Complex)]).unwrap();
        assert!(result.code.contains("complex_mul"));
    }

    #[test]
    fn test_complex_constructor() {
        let result = emit("complex(x, y)", &[("x", Type::Scalar), ("y", Type::Scalar)]).unwrap();
        assert!(result.code.contains("make_float2"));
    }

    #[test]
    fn test_re() {
        let result = emit("re(z)", &[("z", Type::Complex)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert!(result.code.contains(".x"));
    }

    #[test]
    fn test_im() {
        let result = emit("im(z)", &[("z", Type::Complex)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert!(result.code.contains(".y"));
    }

    #[test]
    fn test_conj() {
        let result = emit("conj(z)", &[("z", Type::Complex)]).unwrap();
        assert_eq!(result.typ, Type::Complex);
    }

    #[test]
    fn test_abs() {
        let result = emit("abs(z)", &[("z", Type::Complex)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert!(result.code.contains("sqrtf"));
    }
}
