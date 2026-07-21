//! HIP code generation for complex number expressions.
//!
//! Uses HIP's float2 for complex numbers (.x = real, .y = imag).
//! HIP is source-compatible with CUDA, using the same syntax and functions.

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
        Type::Complex => "float2",
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
            let bool_expr = cond::emit_and(&l_bool, &r_bool);
            Ok(HIPExpr {
                code: cond::bool_to_scalar(&bool_expr),
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
            let bool_expr = cond::emit_or(&l_bool, &r_bool);
            Ok(HIPExpr {
                code: cond::bool_to_scalar(&bool_expr),
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
        BinOp::Add => {
            let typ = if left.typ == Type::Complex || right.typ == Type::Complex {
                Type::Complex
            } else {
                Type::Scalar
            };
            Ok(HIPExpr {
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
            Ok(HIPExpr {
                code: format!("({} - {})", left.code, right.code),
                typ,
            })
        }
        BinOp::Mul => match (left.typ, right.typ) {
            (Type::Scalar, Type::Scalar) => Ok(HIPExpr {
                code: format!("({} * {})", left.code, right.code),
                typ: Type::Scalar,
            }),
            (Type::Scalar, Type::Complex) | (Type::Complex, Type::Scalar) => Ok(HIPExpr {
                code: format!("({} * {})", left.code, right.code),
                typ: Type::Complex,
            }),
            (Type::Complex, Type::Complex) => Ok(HIPExpr {
                code: format!("complex_mul({}, {})", left.code, right.code),
                typ: Type::Complex,
            }),
        },
        BinOp::Div => match (left.typ, right.typ) {
            (Type::Scalar, Type::Scalar) => Ok(HIPExpr {
                code: format!("({} / {})", left.code, right.code),
                typ: Type::Scalar,
            }),
            (Type::Complex, Type::Scalar) => Ok(HIPExpr {
                code: format!("({} / {})", left.code, right.code),
                typ: Type::Complex,
            }),
            (Type::Complex, Type::Complex) => Ok(HIPExpr {
                code: format!("complex_div({}, {})", left.code, right.code),
                typ: Type::Complex,
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
                Ok(HIPExpr {
                    code: format!("complex_pow({}, {})", left.code, right.code),
                    typ: Type::Complex,
                })
            }
        }
        _ => Err(HIPError::TypeMismatch {
            op: "unsupported",
            left: left.typ,
            right: right.typ,
        }),
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
        UnaryOp::BitNot => Err(HIPError::TypeMismatch {
            op: "~",
            left: inner.typ,
            right: inner.typ,
        }),
    }
}

fn emit_function_call(name: &str, args: Vec<HIPExpr>) -> Result<HIPExpr, HIPError> {
    match name {
        "complex" => {
            if args.len() != 2 {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            Ok(HIPExpr {
                code: format!("make_float2({}, {})", args[0].code, args[1].code),
                typ: Type::Complex,
            })
        }
        "polar" => {
            if args.len() != 2 {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            // polar(r, theta) = r * (cos(theta) + i*sin(theta))
            Ok(HIPExpr {
                code: format!(
                    "make_float2({} * cosf({}), {} * sinf({}))",
                    args[0].code, args[1].code, args[0].code, args[1].code
                ),
                typ: Type::Complex,
            })
        }
        "re" => {
            if args.len() != 1 || args[0].typ != Type::Complex {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            Ok(HIPExpr {
                code: format!("{}.x", args[0].code),
                typ: Type::Scalar,
            })
        }
        "im" => {
            if args.len() != 1 || args[0].typ != Type::Complex {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            Ok(HIPExpr {
                code: format!("{}.y", args[0].code),
                typ: Type::Scalar,
            })
        }
        "conj" => {
            if args.len() != 1 || args[0].typ != Type::Complex {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            Ok(HIPExpr {
                code: format!("make_float2({}.x, -{}.y)", args[0].code, args[0].code),
                typ: Type::Complex,
            })
        }
        "abs" => {
            if args.len() != 1 {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(HIPExpr {
                    code: format!("fabsf({})", args[0].code),
                    typ: Type::Scalar,
                }),
                Type::Complex => Ok(HIPExpr {
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
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            Ok(HIPExpr {
                code: format!("atan2f({}.y, {}.x)", args[0].code, args[0].code),
                typ: Type::Scalar,
            })
        }
        "norm" => {
            if args.len() != 1 || args[0].typ != Type::Complex {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            Ok(HIPExpr {
                code: format!(
                    "({}.x * {}.x + {}.y * {}.y)",
                    args[0].code, args[0].code, args[0].code, args[0].code
                ),
                typ: Type::Scalar,
            })
        }
        "exp" => {
            if args.len() != 1 {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(HIPExpr {
                    code: format!("expf({})", args[0].code),
                    typ: Type::Scalar,
                }),
                Type::Complex => Ok(HIPExpr {
                    code: format!("complex_exp({})", args[0].code),
                    typ: Type::Complex,
                }),
            }
        }
        "log" => {
            if args.len() != 1 {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(HIPExpr {
                    code: format!("logf({})", args[0].code),
                    typ: Type::Scalar,
                }),
                Type::Complex => Ok(HIPExpr {
                    code: format!("complex_log({})", args[0].code),
                    typ: Type::Complex,
                }),
            }
        }
        "sqrt" => {
            if args.len() != 1 {
                return Err(HIPError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(HIPExpr {
                    code: format!("sqrtf({})", args[0].code),
                    typ: Type::Scalar,
                }),
                Type::Complex => Ok(HIPExpr {
                    code: format!("complex_sqrt({})", args[0].code),
                    typ: Type::Complex,
                }),
            }
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
