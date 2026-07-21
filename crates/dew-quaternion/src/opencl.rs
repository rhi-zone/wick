//! OpenCL code generation for quaternion expressions.
//!
//! Uses OpenCL built-in vector types:
//! - `float4` for quaternions (x, y, z, w components)
//! - `float3` for 3D vectors
//!
//! Uses OpenCL built-in functions where available:
//! - `dot()`, `length()`, `normalize()` for vectors
//! - Quaternion-specific operations require external functions

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
    /// Conditionals require scalar types.
    UnsupportedTypeForConditional(Type),
    /// Operation not supported for this type.
    UnsupportedOperation(&'static str),
    /// Feature not supported in expression-only codegen.
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
                write!(f, "unsupported operation for quaternion: {op}")
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
        Type::Vec3 => "float3",
        Type::Quaternion => "float4",
    }
}

/// Result of OpenCL emission: code string and its type.
pub struct OpenCLExpr {
    pub code: String,
    pub typ: Type,
}

/// Result of full OpenCL emission with accumulated statements.
struct Emission {
    statements: Vec<String>,
    expr: String,
    typ: Type,
}

/// Format a numeric literal for OpenCL.
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

        Ast::Call(name, args) => {
            let arg_exprs: Vec<OpenCLExpr> = args
                .iter()
                .map(|a| emit_opencl(a, var_types))
                .collect::<Result<_, _>>()?;
            emit_function_call(name, arg_exprs)
        }

        Ast::Compare(op, left, right) => {
            let left_expr = emit_opencl(left, var_types)?;
            let right_expr = emit_opencl(right, var_types)?;
            if left_expr.typ != Type::Scalar || right_expr.typ != Type::Scalar {
                return Err(OpenCLError::UnsupportedTypeForConditional(left_expr.typ));
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
                return Err(OpenCLError::UnsupportedTypeForConditional(left_expr.typ));
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
                return Err(OpenCLError::UnsupportedTypeForConditional(left_expr.typ));
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

        Ast::Let { .. } => {
            let emission = emit_full(ast, var_types)?;
            if !emission.statements.is_empty() {
                return Err(OpenCLError::UnsupportedFeature(
                    "let bindings in expression context (use emit_opencl_fn)".to_string(),
                ));
            }
            Ok(OpenCLExpr {
                code: emission.expr,
                typ: emission.typ,
            })
        }
    }
}

/// Emit OpenCL with full statement support.
fn emit_full(ast: &Ast, var_types: &HashMap<String, Type>) -> Result<Emission, OpenCLError> {
    match ast {
        Ast::Let { name, value, body } => {
            let value_emission = emit_full(value, var_types)?;
            let mut new_var_types = var_types.clone();
            new_var_types.insert(name.clone(), value_emission.typ);
            let body_emission = emit_full(body, &new_var_types)?;

            let mut statements = value_emission.statements;
            statements.push(format!(
                "{} {} = {};",
                type_to_opencl(value_emission.typ),
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
            let result = emit_opencl(ast, var_types)?;
            Ok(Emission {
                statements: vec![],
                expr: result.code,
                typ: result.typ,
            })
        }
    }
}

/// Emit a complete OpenCL kernel function with let statement support.
pub fn emit_opencl_fn(
    name: &str,
    ast: &Ast,
    params: &[(&str, Type)],
    return_type: Type,
) -> Result<String, OpenCLError> {
    let var_types: HashMap<String, Type> =
        params.iter().map(|(n, t)| (n.to_string(), *t)).collect();
    let emission = emit_full(ast, &var_types)?;

    let params_str = params
        .iter()
        .map(|(n, t)| format!("{} {}", type_to_opencl(*t), n))
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
        type_to_opencl(return_type),
        name,
        params_str,
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
        // OpenCL supports + for scalar, float3, and float4
        (Type::Scalar, Type::Scalar)
        | (Type::Vec3, Type::Vec3)
        | (Type::Quaternion, Type::Quaternion) => Ok(OpenCLExpr {
            code: format!("({} + {})", left.code, right.code),
            typ: left.typ,
        }),
        _ => Err(OpenCLError::TypeMismatch {
            op: "+",
            left: left.typ,
            right: right.typ,
        }),
    }
}

fn emit_sub(left: OpenCLExpr, right: OpenCLExpr) -> Result<OpenCLExpr, OpenCLError> {
    match (left.typ, right.typ) {
        // OpenCL supports - for scalar, float3, and float4
        (Type::Scalar, Type::Scalar)
        | (Type::Vec3, Type::Vec3)
        | (Type::Quaternion, Type::Quaternion) => Ok(OpenCLExpr {
            code: format!("({} - {})", left.code, right.code),
            typ: left.typ,
        }),
        _ => Err(OpenCLError::TypeMismatch {
            op: "-",
            left: left.typ,
            right: right.typ,
        }),
    }
}

fn emit_mul(left: OpenCLExpr, right: OpenCLExpr) -> Result<OpenCLExpr, OpenCLError> {
    match (left.typ, right.typ) {
        // Scalar * Scalar
        (Type::Scalar, Type::Scalar) => Ok(OpenCLExpr {
            code: format!("({} * {})", left.code, right.code),
            typ: Type::Scalar,
        }),

        // Scalar * Vec3 or Vec3 * Scalar (OpenCL supports broadcast)
        (Type::Scalar, Type::Vec3) => Ok(OpenCLExpr {
            code: format!("({} * {})", left.code, right.code),
            typ: Type::Vec3,
        }),
        (Type::Vec3, Type::Scalar) => Ok(OpenCLExpr {
            code: format!("({} * {})", left.code, right.code),
            typ: Type::Vec3,
        }),

        // Scalar * Quaternion or Quaternion * Scalar (OpenCL supports broadcast)
        (Type::Scalar, Type::Quaternion) => Ok(OpenCLExpr {
            code: format!("({} * {})", left.code, right.code),
            typ: Type::Quaternion,
        }),
        (Type::Quaternion, Type::Scalar) => Ok(OpenCLExpr {
            code: format!("({} * {})", left.code, right.code),
            typ: Type::Quaternion,
        }),

        // Quaternion * Quaternion (Hamilton product) - requires external function
        (Type::Quaternion, Type::Quaternion) => Ok(OpenCLExpr {
            code: format!("quat_mul({}, {})", left.code, right.code),
            typ: Type::Quaternion,
        }),

        // Quaternion * Vec3 (rotate vector) - requires external function
        (Type::Quaternion, Type::Vec3) => Ok(OpenCLExpr {
            code: format!("quat_mul_vec3({}, {})", left.code, right.code),
            typ: Type::Vec3,
        }),

        _ => Err(OpenCLError::TypeMismatch {
            op: "*",
            left: left.typ,
            right: right.typ,
        }),
    }
}

fn emit_div(left: OpenCLExpr, right: OpenCLExpr) -> Result<OpenCLExpr, OpenCLError> {
    match (left.typ, right.typ) {
        // OpenCL supports / for scalar with scalar, vector with scalar
        (Type::Scalar, Type::Scalar) => Ok(OpenCLExpr {
            code: format!("({} / {})", left.code, right.code),
            typ: Type::Scalar,
        }),
        (Type::Vec3, Type::Scalar) => Ok(OpenCLExpr {
            code: format!("({} / {})", left.code, right.code),
            typ: Type::Vec3,
        }),
        (Type::Quaternion, Type::Scalar) => Ok(OpenCLExpr {
            code: format!("({} / {})", left.code, right.code),
            typ: Type::Quaternion,
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
        _ => Err(OpenCLError::TypeMismatch {
            op: "^",
            left: base.typ,
            right: exp.typ,
        }),
    }
}

fn emit_unaryop(op: UnaryOp, inner: OpenCLExpr) -> Result<OpenCLExpr, OpenCLError> {
    match op {
        UnaryOp::Neg => {
            // OpenCL supports unary negation for scalar, float3, float4
            Ok(OpenCLExpr {
                code: format!("(-{})", inner.code),
                typ: inner.typ,
            })
        }
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
        "conj" => {
            if args.len() != 1 || args[0].typ != Type::Quaternion {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            // Quaternion conjugate: (x, y, z, w) -> (-x, -y, -z, w)
            Ok(OpenCLExpr {
                code: format!("quat_conj({})", args[0].code),
                typ: Type::Quaternion,
            })
        }

        "length" => {
            if args.len() != 1 {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(OpenCLExpr {
                    code: format!("fabs({})", args[0].code),
                    typ: Type::Scalar,
                }),
                // OpenCL has built-in length() for float3 and float4
                Type::Vec3 | Type::Quaternion => Ok(OpenCLExpr {
                    code: format!("length({})", args[0].code),
                    typ: Type::Scalar,
                }),
            }
        }

        "normalize" => {
            if args.len() != 1 {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Err(OpenCLError::UnknownFunction(name.to_string())),
                // OpenCL has built-in normalize() for float3 and float4
                Type::Vec3 | Type::Quaternion => Ok(OpenCLExpr {
                    code: format!("normalize({})", args[0].code),
                    typ: args[0].typ,
                }),
            }
        }

        "inverse" => {
            if args.len() != 1 || args[0].typ != Type::Quaternion {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            Ok(OpenCLExpr {
                code: format!("quat_inverse({})", args[0].code),
                typ: Type::Quaternion,
            })
        }

        "dot" => {
            if args.len() != 2 || args[0].typ != args[1].typ {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Err(OpenCLError::UnknownFunction(name.to_string())),
                // OpenCL has built-in dot() for float3 and float4
                Type::Vec3 | Type::Quaternion => Ok(OpenCLExpr {
                    code: format!("dot({}, {})", args[0].code, args[1].code),
                    typ: Type::Scalar,
                }),
            }
        }

        "lerp" | "mix" => {
            if args.len() != 3 || args[2].typ != Type::Scalar {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                // OpenCL has built-in mix() for scalar, float3, float4
                Type::Scalar | Type::Vec3 | Type::Quaternion => Ok(OpenCLExpr {
                    code: format!("mix({}, {}, {})", args[0].code, args[1].code, args[2].code),
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
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            Ok(OpenCLExpr {
                code: format!(
                    "quat_slerp({}, {}, {})",
                    args[0].code, args[1].code, args[2].code
                ),
                typ: Type::Quaternion,
            })
        }

        "axis_angle" => {
            if args.len() != 2 || args[0].typ != Type::Vec3 || args[1].typ != Type::Scalar {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            Ok(OpenCLExpr {
                code: format!("quat_from_axis_angle({}, {})", args[0].code, args[1].code),
                typ: Type::Quaternion,
            })
        }

        "rotate" => {
            if args.len() != 2 || args[0].typ != Type::Vec3 || args[1].typ != Type::Quaternion {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            Ok(OpenCLExpr {
                code: format!("quat_mul_vec3({}, {})", args[1].code, args[0].code),
                typ: Type::Vec3,
            })
        }

        "cross" => {
            if args.len() != 2 || args[0].typ != Type::Vec3 || args[1].typ != Type::Vec3 {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            // OpenCL has built-in cross() for float3
            Ok(OpenCLExpr {
                code: format!("cross({}, {})", args[0].code, args[1].code),
                typ: Type::Vec3,
            })
        }

        "distance" => {
            if args.len() != 2 || args[0].typ != args[1].typ {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Err(OpenCLError::UnknownFunction(name.to_string())),
                // OpenCL has built-in distance() for float3 and float4
                Type::Vec3 | Type::Quaternion => Ok(OpenCLExpr {
                    code: format!("distance({}, {})", args[0].code, args[1].code),
                    typ: Type::Scalar,
                }),
            }
        }

        "vec3" => {
            if args.len() != 3 || args.iter().any(|a| a.typ != Type::Scalar) {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            Ok(OpenCLExpr {
                code: format!(
                    "(float3)({}, {}, {})",
                    args[0].code, args[1].code, args[2].code
                ),
                typ: Type::Vec3,
            })
        }

        "quat" => {
            if args.len() != 4 || args.iter().any(|a| a.typ != Type::Scalar) {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            Ok(OpenCLExpr {
                code: format!(
                    "(float4)({}, {}, {}, {})",
                    args[0].code, args[1].code, args[2].code, args[3].code
                ),
                typ: Type::Quaternion,
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
    fn test_quaternion_add() {
        let result = emit("a + b", &[("a", Type::Quaternion), ("b", Type::Quaternion)]).unwrap();
        assert_eq!(result.typ, Type::Quaternion);
        assert_eq!(result.code, "(a + b)");
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
    fn test_scalar_quat_mul() {
        let result = emit("s * q", &[("s", Type::Scalar), ("q", Type::Quaternion)]).unwrap();
        assert_eq!(result.typ, Type::Quaternion);
        assert_eq!(result.code, "(s * q)");
    }

    #[test]
    fn test_normalize() {
        let result = emit("normalize(q)", &[("q", Type::Quaternion)]).unwrap();
        assert_eq!(result.typ, Type::Quaternion);
        assert_eq!(result.code, "normalize(q)");
    }

    #[test]
    fn test_normalize_vec3() {
        let result = emit("normalize(v)", &[("v", Type::Vec3)]).unwrap();
        assert_eq!(result.typ, Type::Vec3);
        assert_eq!(result.code, "normalize(v)");
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
        assert_eq!(result.code, "dot(a, b)");
    }

    #[test]
    fn test_dot_vec3() {
        let result = emit("dot(a, b)", &[("a", Type::Vec3), ("b", Type::Vec3)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert_eq!(result.code, "dot(a, b)");
    }

    #[test]
    fn test_length() {
        let result = emit("length(q)", &[("q", Type::Quaternion)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert_eq!(result.code, "length(q)");
    }

    #[test]
    fn test_cross() {
        let result = emit("cross(a, b)", &[("a", Type::Vec3), ("b", Type::Vec3)]).unwrap();
        assert_eq!(result.typ, Type::Vec3);
        assert_eq!(result.code, "cross(a, b)");
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
    fn test_mix() {
        let result = emit(
            "mix(a, b, t)",
            &[
                ("a", Type::Quaternion),
                ("b", Type::Quaternion),
                ("t", Type::Scalar),
            ],
        )
        .unwrap();
        assert_eq!(result.typ, Type::Quaternion);
        assert!(result.code.contains("mix"));
    }

    #[test]
    fn test_vec3_constructor() {
        let result = emit(
            "vec3(x, y, z)",
            &[
                ("x", Type::Scalar),
                ("y", Type::Scalar),
                ("z", Type::Scalar),
            ],
        )
        .unwrap();
        assert_eq!(result.typ, Type::Vec3);
        assert!(result.code.contains("(float3)"));
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
        assert_eq!(result.typ, Type::Quaternion);
        assert!(result.code.contains("(float4)"));
    }

    #[test]
    fn test_emit_opencl_fn_simple() {
        let expr = Expr::parse("normalize(q)").unwrap();
        let code = emit_opencl_fn(
            "norm_quat",
            expr.ast(),
            &[("q", Type::Quaternion)],
            Type::Quaternion,
        )
        .unwrap();
        assert!(code.contains("float4 norm_quat(float4 q)"));
        assert!(code.contains("normalize(q)"));
    }

    #[test]
    fn test_emit_opencl_fn_with_let() {
        let expr = Expr::parse("let sq = q * q; sq + q").unwrap();
        let code = emit_opencl_fn(
            "square_add",
            expr.ast(),
            &[("q", Type::Quaternion)],
            Type::Quaternion,
        )
        .unwrap();
        assert!(code.contains("float4 sq ="));
        assert!(code.contains("(sq + q)"));
    }

    #[test]
    fn test_negation() {
        let result = emit("-q", &[("q", Type::Quaternion)]).unwrap();
        assert_eq!(result.typ, Type::Quaternion);
        assert_eq!(result.code, "(-q)");
    }
}
