//! OpenCL code generation for complex expressions.
//!
//! Complex numbers are represented using float2: .x = real, .y = imaginary.
//! Uses function-based operations (complex_add, complex_mul, etc.) that assume
//! helper functions are defined.

use crate::Type;
use dew_cond::opencl as cond;
use dew_core::{Ast, BinOp, UnaryOp};
use std::collections::HashMap;

/// Error during OpenCL code generation.
#[derive(Debug, Clone, PartialEq)]
pub enum OpenCLError {
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

impl std::fmt::Display for OpenCLError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenCLError::UnknownVariable(name) => write!(f, "unknown variable: '{name}'"),
            OpenCLError::UnknownFunction(name) => write!(f, "unknown function: '{name}'"),
            OpenCLError::TypeMismatch { op, left, right } => {
                write!(f, "type mismatch for {op}: {left} vs {right}")
            }
            OpenCLError::UnsupportedTypeForConditional(t) => {
                write!(f, "conditionals require scalar type, got {t}")
            }
            OpenCLError::UnsupportedOperation(op) => {
                write!(f, "unsupported operation for complex: {op}")
            }
            OpenCLError::UnsupportedFeature(feat) => {
                write!(f, "unsupported feature in OpenCL codegen: {feat}")
            }
        }
    }
}

impl std::error::Error for OpenCLError {}

/// Convert a Type to its OpenCL representation.
pub fn type_to_opencl(t: Type) -> &'static str {
    match t {
        Type::Scalar => "float",
        Type::Complex => "float2", // .x = real, .y = imag
    }
}

/// Result of OpenCL emission: code string and its type.
pub struct OpenCLExpr {
    pub code: String,
    pub typ: Type,
}

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

fn format_literal(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{:.1}f", n)
    } else {
        format!("{}f", n)
    }
}

/// Emit OpenCL code for an AST with type propagation.
pub fn emit_opencl(
    ast: &Ast,
    var_types: &HashMap<String, Type>,
) -> Result<OpenCLExpr, OpenCLError> {
    match ast {
        Ast::Num(n) => Ok(OpenCLExpr {
            code: format_literal(*n),
            typ: Type::Scalar,
        }),

        Ast::Var(name) => {
            let typ = var_types
                .get(name)
                .copied()
                .ok_or_else(|| OpenCLError::UnknownVariable(name.clone()))?;
            Ok(OpenCLExpr {
                code: name.clone(),
                typ,
            })
        }

        Ast::BinOp(op, left, right) => {
            let left_expr = emit_opencl(left, var_types)?;
            let right_expr = emit_opencl(right, var_types)?;
            emit_binop(*op, left_expr, right_expr)
        }

        Ast::UnaryOp(op, inner) => {
            let inner_expr = emit_opencl(inner, var_types)?;
            emit_unaryop(*op, inner_expr)
        }

        Ast::Compare(op, left, right) => {
            let left_expr = emit_opencl(left, var_types)?;
            let right_expr = emit_opencl(right, var_types)?;
            if left_expr.typ != Type::Scalar || right_expr.typ != Type::Scalar {
                return Err(OpenCLError::UnsupportedTypeForConditional(Type::Complex));
            }
            let bool_expr = cond::emit_compare(*op, &left_expr.code, &right_expr.code);
            Ok(OpenCLExpr {
                code: cond::bool_to_scalar(&bool_expr),
                typ: Type::Scalar,
            })
        }

        Ast::And(left, right) => {
            let left_expr = emit_opencl(left, var_types)?;
            let right_expr = emit_opencl(right, var_types)?;
            if left_expr.typ != Type::Scalar || right_expr.typ != Type::Scalar {
                return Err(OpenCLError::UnsupportedTypeForConditional(Type::Complex));
            }
            let l_bool = cond::scalar_to_bool(&left_expr.code);
            let r_bool = cond::scalar_to_bool(&right_expr.code);
            let bool_expr = cond::emit_and(&l_bool, &r_bool);
            Ok(OpenCLExpr {
                code: cond::bool_to_scalar(&bool_expr),
                typ: Type::Scalar,
            })
        }

        Ast::Or(left, right) => {
            let left_expr = emit_opencl(left, var_types)?;
            let right_expr = emit_opencl(right, var_types)?;
            if left_expr.typ != Type::Scalar || right_expr.typ != Type::Scalar {
                return Err(OpenCLError::UnsupportedTypeForConditional(Type::Complex));
            }
            let l_bool = cond::scalar_to_bool(&left_expr.code);
            let r_bool = cond::scalar_to_bool(&right_expr.code);
            let bool_expr = cond::emit_or(&l_bool, &r_bool);
            Ok(OpenCLExpr {
                code: cond::bool_to_scalar(&bool_expr),
                typ: Type::Scalar,
            })
        }

        Ast::If(cond_ast, then_ast, else_ast) => {
            let cond_expr = emit_opencl(cond_ast, var_types)?;
            let then_expr = emit_opencl(then_ast, var_types)?;
            let else_expr = emit_opencl(else_ast, var_types)?;
            if cond_expr.typ != Type::Scalar {
                return Err(OpenCLError::UnsupportedTypeForConditional(cond_expr.typ));
            }
            if then_expr.typ != else_expr.typ {
                return Err(OpenCLError::TypeMismatch {
                    op: "if/else",
                    left: then_expr.typ,
                    right: else_expr.typ,
                });
            }
            let cond_bool = cond::scalar_to_bool(&cond_expr.code);
            Ok(OpenCLExpr {
                code: cond::emit_if(&cond_bool, &then_expr.code, &else_expr.code),
                typ: then_expr.typ,
            })
        }

        Ast::Call(name, args) => {
            let arg_exprs: Vec<OpenCLExpr> = args
                .iter()
                .map(|a| emit_opencl(a, var_types))
                .collect::<Result<_, _>>()?;
            emit_function_call(name, arg_exprs)
        }

        Ast::Let { .. } => {
            let emission = emit_full(ast, var_types)?;
            if emission.statements.is_empty() {
                Ok(OpenCLExpr {
                    code: emission.expr,
                    typ: emission.typ,
                })
            } else {
                Err(OpenCLError::UnsupportedFeature(
                    "let in expression position (use emit_opencl_fn for full support)".to_string(),
                ))
            }
        }
    }
}

fn emit_full(ast: &Ast, var_types: &HashMap<String, Type>) -> Result<Emission, OpenCLError> {
    match ast {
        Ast::Let { name, value, body } => {
            let value_emission = emit_full(value, var_types)?;
            let mut new_var_types = var_types.clone();
            new_var_types.insert(name.clone(), value_emission.typ);
            let mut body_emission = emit_full(body, &new_var_types)?;

            let mut statements = value_emission.statements;
            statements.push(format!(
                "{} {} = {};",
                type_to_opencl(value_emission.typ),
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
            let result = emit_opencl(ast, var_types)?;
            Ok(Emission::expr_only(result.code, result.typ))
        }
    }
}

/// Emit a complete OpenCL function.
pub fn emit_opencl_fn(
    name: &str,
    ast: &Ast,
    params: &[(&str, Type)],
    return_type: Type,
) -> Result<String, OpenCLError> {
    let var_types: HashMap<String, Type> =
        params.iter().map(|(n, t)| (n.to_string(), *t)).collect();
    let emission = emit_full(ast, &var_types)?;

    let param_list: Vec<String> = params
        .iter()
        .map(|(n, t)| format!("{} {}", type_to_opencl(*t), n))
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
        type_to_opencl(return_type),
        name,
        param_list.join(", "),
        body
    ))
}

fn emit_binop(op: BinOp, left: OpenCLExpr, right: OpenCLExpr) -> Result<OpenCLExpr, OpenCLError> {
    match op {
        BinOp::Add => emit_add(left, right),
        BinOp::Sub => emit_sub(left, right),
        BinOp::Mul => emit_mul(left, right),
        BinOp::Div => emit_div(left, right),
        BinOp::Pow => emit_pow(left, right),
        BinOp::Rem => Err(OpenCLError::UnsupportedOperation("%")),
        BinOp::BitAnd => Err(OpenCLError::UnsupportedOperation("&")),
        BinOp::BitOr => Err(OpenCLError::UnsupportedOperation("|")),
        BinOp::Shl => Err(OpenCLError::UnsupportedOperation("<<")),
        BinOp::Shr => Err(OpenCLError::UnsupportedOperation(">>")),
    }
}

fn emit_add(left: OpenCLExpr, right: OpenCLExpr) -> Result<OpenCLExpr, OpenCLError> {
    match (left.typ, right.typ) {
        (Type::Scalar, Type::Scalar) => Ok(OpenCLExpr {
            code: format!("({} + {})", left.code, right.code),
            typ: Type::Scalar,
        }),
        // float2 + float2 works in OpenCL
        (Type::Complex, Type::Complex) => Ok(OpenCLExpr {
            code: format!("({} + {})", left.code, right.code),
            typ: Type::Complex,
        }),
        (Type::Scalar, Type::Complex) => Ok(OpenCLExpr {
            code: format!("((float2)({}, 0.0f) + {})", left.code, right.code),
            typ: Type::Complex,
        }),
        (Type::Complex, Type::Scalar) => Ok(OpenCLExpr {
            code: format!("({} + (float2)({}, 0.0f))", left.code, right.code),
            typ: Type::Complex,
        }),
    }
}

fn emit_sub(left: OpenCLExpr, right: OpenCLExpr) -> Result<OpenCLExpr, OpenCLError> {
    match (left.typ, right.typ) {
        (Type::Scalar, Type::Scalar) => Ok(OpenCLExpr {
            code: format!("({} - {})", left.code, right.code),
            typ: Type::Scalar,
        }),
        (Type::Complex, Type::Complex) => Ok(OpenCLExpr {
            code: format!("({} - {})", left.code, right.code),
            typ: Type::Complex,
        }),
        (Type::Scalar, Type::Complex) => Ok(OpenCLExpr {
            code: format!("((float2)({}, 0.0f) - {})", left.code, right.code),
            typ: Type::Complex,
        }),
        (Type::Complex, Type::Scalar) => Ok(OpenCLExpr {
            code: format!("({} - (float2)({}, 0.0f))", left.code, right.code),
            typ: Type::Complex,
        }),
    }
}

fn emit_mul(left: OpenCLExpr, right: OpenCLExpr) -> Result<OpenCLExpr, OpenCLError> {
    match (left.typ, right.typ) {
        (Type::Scalar, Type::Scalar) => Ok(OpenCLExpr {
            code: format!("({} * {})", left.code, right.code),
            typ: Type::Scalar,
        }),
        (Type::Scalar, Type::Complex) => Ok(OpenCLExpr {
            code: format!("({} * {})", left.code, right.code),
            typ: Type::Complex,
        }),
        (Type::Complex, Type::Scalar) => Ok(OpenCLExpr {
            code: format!("({} * {})", left.code, right.code),
            typ: Type::Complex,
        }),
        // Complex multiplication: (a+bi)(c+di) = (ac-bd) + (ad+bc)i
        (Type::Complex, Type::Complex) => Ok(OpenCLExpr {
            code: format!("complex_mul({}, {})", left.code, right.code),
            typ: Type::Complex,
        }),
    }
}

fn emit_div(left: OpenCLExpr, right: OpenCLExpr) -> Result<OpenCLExpr, OpenCLError> {
    match (left.typ, right.typ) {
        (Type::Scalar, Type::Scalar) => Ok(OpenCLExpr {
            code: format!("({} / {})", left.code, right.code),
            typ: Type::Scalar,
        }),
        (Type::Complex, Type::Scalar) => Ok(OpenCLExpr {
            code: format!("({} / {})", left.code, right.code),
            typ: Type::Complex,
        }),
        (Type::Complex, Type::Complex) => Ok(OpenCLExpr {
            code: format!("complex_div({}, {})", left.code, right.code),
            typ: Type::Complex,
        }),
        _ => Err(OpenCLError::TypeMismatch {
            op: "/",
            left: left.typ,
            right: right.typ,
        }),
    }
}

fn emit_pow(base: OpenCLExpr, exp: OpenCLExpr) -> Result<OpenCLExpr, OpenCLError> {
    match (base.typ, exp.typ) {
        (Type::Scalar, Type::Scalar) => Ok(OpenCLExpr {
            code: format!("pow({}, {})", base.code, exp.code),
            typ: Type::Scalar,
        }),
        (Type::Complex, Type::Scalar) => Ok(OpenCLExpr {
            code: format!("complex_powf({}, {})", base.code, exp.code),
            typ: Type::Complex,
        }),
        (Type::Complex, Type::Complex) => Ok(OpenCLExpr {
            code: format!("complex_pow({}, {})", base.code, exp.code),
            typ: Type::Complex,
        }),
        _ => Err(OpenCLError::TypeMismatch {
            op: "^",
            left: base.typ,
            right: exp.typ,
        }),
    }
}

fn emit_unaryop(op: UnaryOp, inner: OpenCLExpr) -> Result<OpenCLExpr, OpenCLError> {
    match op {
        UnaryOp::Neg => Ok(OpenCLExpr {
            code: format!("(-{})", inner.code),
            typ: inner.typ,
        }),
        UnaryOp::Not => {
            if inner.typ != Type::Scalar {
                return Err(OpenCLError::UnsupportedTypeForConditional(inner.typ));
            }
            let bool_expr = cond::scalar_to_bool(&inner.code);
            Ok(OpenCLExpr {
                code: cond::bool_to_scalar(&cond::emit_not(&bool_expr)),
                typ: Type::Scalar,
            })
        }
        UnaryOp::BitNot => Err(OpenCLError::UnsupportedOperation("~")),
    }
}

fn emit_function_call(name: &str, args: Vec<OpenCLExpr>) -> Result<OpenCLExpr, OpenCLError> {
    match name {
        "re" => {
            if args.len() != 1 || args[0].typ != Type::Complex {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            Ok(OpenCLExpr {
                code: format!("{}.x", args[0].code),
                typ: Type::Scalar,
            })
        }

        "im" => {
            if args.len() != 1 || args[0].typ != Type::Complex {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            Ok(OpenCLExpr {
                code: format!("{}.y", args[0].code),
                typ: Type::Scalar,
            })
        }

        "conj" => {
            if args.len() != 1 || args[0].typ != Type::Complex {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            // conj(a+bi) = a-bi -> flip sign of .y
            Ok(OpenCLExpr {
                code: format!("(float2)({}.x, -{}.y)", args[0].code, args[0].code),
                typ: Type::Complex,
            })
        }

        "abs" => {
            if args.len() != 1 {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(OpenCLExpr {
                    code: format!("fabs({})", args[0].code),
                    typ: Type::Scalar,
                }),
                // |z| = sqrt(x^2 + y^2) = length of float2
                Type::Complex => Ok(OpenCLExpr {
                    code: format!("length({})", args[0].code),
                    typ: Type::Scalar,
                }),
            }
        }

        "arg" => {
            if args.len() != 1 || args[0].typ != Type::Complex {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            Ok(OpenCLExpr {
                code: format!("atan2({}.y, {}.x)", args[0].code, args[0].code),
                typ: Type::Scalar,
            })
        }

        "norm" => {
            if args.len() != 1 || args[0].typ != Type::Complex {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            // norm = |z|^2 = dot(z, z)
            Ok(OpenCLExpr {
                code: format!("dot({}, {})", args[0].code, args[0].code),
                typ: Type::Scalar,
            })
        }

        "exp" => {
            if args.len() != 1 {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(OpenCLExpr {
                    code: format!("exp({})", args[0].code),
                    typ: Type::Scalar,
                }),
                Type::Complex => Ok(OpenCLExpr {
                    code: format!("complex_exp({})", args[0].code),
                    typ: Type::Complex,
                }),
            }
        }

        "log" => {
            if args.len() != 1 {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(OpenCLExpr {
                    code: format!("log({})", args[0].code),
                    typ: Type::Scalar,
                }),
                Type::Complex => Ok(OpenCLExpr {
                    code: format!("complex_log({})", args[0].code),
                    typ: Type::Complex,
                }),
            }
        }

        "sqrt" => {
            if args.len() != 1 {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(OpenCLExpr {
                    code: format!("sqrt({})", args[0].code),
                    typ: Type::Scalar,
                }),
                Type::Complex => Ok(OpenCLExpr {
                    code: format!("complex_sqrt({})", args[0].code),
                    typ: Type::Complex,
                }),
            }
        }

        "polar" => {
            if args.len() != 2 || args[0].typ != Type::Scalar || args[1].typ != Type::Scalar {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            // polar(r, theta) = r * e^(i*theta) = r * (cos(theta) + i*sin(theta))
            Ok(OpenCLExpr {
                code: format!(
                    "(float2)({} * cos({}), {} * sin({}))",
                    args[0].code, args[1].code, args[0].code, args[1].code
                ),
                typ: Type::Complex,
            })
        }

        "complex" => {
            if args.len() != 2 || args[0].typ != Type::Scalar || args[1].typ != Type::Scalar {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            Ok(OpenCLExpr {
                code: format!("(float2)({}, {})", args[0].code, args[1].code),
                typ: Type::Complex,
            })
        }

        _ => Err(OpenCLError::UnknownFunction(name.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dew_core::Expr;

    fn emit(expr: &str, var_types: &[(&str, Type)]) -> Result<OpenCLExpr, OpenCLError> {
        let expr = Expr::parse(expr).unwrap();
        let types: HashMap<String, Type> =
            var_types.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        emit_opencl(expr.ast(), &types)
    }

    #[test]
    fn test_complex_add() {
        let result = emit("a + b", &[("a", Type::Complex), ("b", Type::Complex)]).unwrap();
        assert_eq!(result.typ, Type::Complex);
        assert!(result.code.contains("+"));
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
        assert!(result.code.contains(".x"));
    }

    #[test]
    fn test_im() {
        let result = emit("im(z)", &[("z", Type::Complex)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert!(result.code.contains(".y"));
    }

    #[test]
    fn test_abs() {
        let result = emit("abs(z)", &[("z", Type::Complex)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert!(result.code.contains("length("));
    }

    #[test]
    fn test_conj() {
        let result = emit("conj(z)", &[("z", Type::Complex)]).unwrap();
        assert_eq!(result.typ, Type::Complex);
        assert!(result.code.contains("-.y") || result.code.contains("-z.y"));
    }

    #[test]
    fn test_polar() {
        let result = emit(
            "polar(r, theta)",
            &[("r", Type::Scalar), ("theta", Type::Scalar)],
        )
        .unwrap();
        assert_eq!(result.typ, Type::Complex);
        assert!(result.code.contains("cos("));
        assert!(result.code.contains("sin("));
    }
}
