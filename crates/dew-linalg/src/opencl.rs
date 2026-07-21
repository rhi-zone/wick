//! OpenCL code generation for linalg expressions.
//!
//! OpenCL has built-in vector types (float2, float3, float4) and operations.
//! Matrix operations use function-based API (matrices aren't built-in).

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
    UnsupportedType(Type),
    UnsupportedTypeForConditional(Type),
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
            OpenCLError::UnsupportedType(t) => write!(f, "unsupported type: {t}"),
            OpenCLError::UnsupportedTypeForConditional(t) => {
                write!(f, "conditionals require scalar type, got {t}")
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
        Type::Vec2 => "float2",
        #[cfg(feature = "3d")]
        Type::Vec3 => "float3",
        #[cfg(feature = "4d")]
        Type::Vec4 => "float4",
        // OpenCL doesn't have built-in matrix types
        Type::Mat2 => "mat2",
        #[cfg(feature = "3d")]
        Type::Mat3 => "mat3",
        #[cfg(feature = "4d")]
        Type::Mat4 => "mat4",
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

/// Generate a complete OpenCL function.
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

fn is_matrix_type(t: Type) -> bool {
    match t {
        Type::Scalar | Type::Vec2 => false,
        Type::Mat2 => true,
        #[cfg(feature = "3d")]
        Type::Vec3 => false,
        #[cfg(feature = "3d")]
        Type::Mat3 => true,
        #[cfg(feature = "4d")]
        Type::Vec4 => false,
        #[cfg(feature = "4d")]
        Type::Mat4 => true,
    }
}

fn emit_binop(op: BinOp, left: OpenCLExpr, right: OpenCLExpr) -> Result<OpenCLExpr, OpenCLError> {
    let result_type = infer_binop_type(op, left.typ, right.typ)?;

    let code = match op {
        BinOp::Add => {
            // OpenCL vectors support + directly
            format!("({} + {})", left.code, right.code)
        }
        BinOp::Sub => {
            format!("({} - {})", left.code, right.code)
        }
        BinOp::Mul => emit_mul(&left, &right, result_type)?,
        BinOp::Div => emit_div(&left, &right)?,
        BinOp::Pow => {
            if left.typ == Type::Scalar && right.typ == Type::Scalar {
                format!("pow({}, {})", left.code, right.code)
            } else {
                return Err(OpenCLError::TypeMismatch {
                    op: "^",
                    left: left.typ,
                    right: right.typ,
                });
            }
        }
        BinOp::Rem => {
            if left.typ == Type::Scalar && right.typ == Type::Scalar {
                format!("fmod({}, {})", left.code, right.code)
            } else {
                return Err(OpenCLError::TypeMismatch {
                    op: "%",
                    left: left.typ,
                    right: right.typ,
                });
            }
        }
        BinOp::BitAnd | BinOp::BitOr | BinOp::Shl | BinOp::Shr => {
            if left.typ == Type::Scalar && right.typ == Type::Scalar {
                let op_str = match op {
                    BinOp::BitAnd => "&",
                    BinOp::BitOr => "|",
                    BinOp::Shl => "<<",
                    BinOp::Shr => ">>",
                    _ => unreachable!(),
                };
                format!("((int){} {} (int){})", left.code, op_str, right.code)
            } else {
                return Err(OpenCLError::TypeMismatch {
                    op: "bitwise",
                    left: left.typ,
                    right: right.typ,
                });
            }
        }
    };

    Ok(OpenCLExpr {
        code,
        typ: result_type,
    })
}

fn infer_binop_type(op: BinOp, left: Type, right: Type) -> Result<Type, OpenCLError> {
    match op {
        BinOp::Add | BinOp::Sub => {
            if left == right {
                Ok(left)
            } else {
                Err(OpenCLError::TypeMismatch {
                    op: if op == BinOp::Add { "+" } else { "-" },
                    left,
                    right,
                })
            }
        }
        BinOp::Mul => infer_mul_type(left, right),
        BinOp::Div => match (left, right) {
            (Type::Scalar, Type::Scalar) => Ok(Type::Scalar),
            (Type::Vec2, Type::Scalar) => Ok(Type::Vec2),
            #[cfg(feature = "3d")]
            (Type::Vec3, Type::Scalar) => Ok(Type::Vec3),
            #[cfg(feature = "4d")]
            (Type::Vec4, Type::Scalar) => Ok(Type::Vec4),
            _ => Err(OpenCLError::TypeMismatch {
                op: "/",
                left,
                right,
            }),
        },
        _ => {
            if left == Type::Scalar && right == Type::Scalar {
                Ok(Type::Scalar)
            } else {
                Err(OpenCLError::TypeMismatch {
                    op: "binop",
                    left,
                    right,
                })
            }
        }
    }
}

fn infer_mul_type(left: Type, right: Type) -> Result<Type, OpenCLError> {
    match (left, right) {
        (Type::Scalar, Type::Scalar) => Ok(Type::Scalar),
        (Type::Vec2, Type::Scalar) | (Type::Scalar, Type::Vec2) => Ok(Type::Vec2),
        #[cfg(feature = "3d")]
        (Type::Vec3, Type::Scalar) | (Type::Scalar, Type::Vec3) => Ok(Type::Vec3),
        #[cfg(feature = "4d")]
        (Type::Vec4, Type::Scalar) | (Type::Scalar, Type::Vec4) => Ok(Type::Vec4),
        (Type::Mat2, Type::Scalar) | (Type::Scalar, Type::Mat2) => Ok(Type::Mat2),
        #[cfg(feature = "3d")]
        (Type::Mat3, Type::Scalar) | (Type::Scalar, Type::Mat3) => Ok(Type::Mat3),
        #[cfg(feature = "4d")]
        (Type::Mat4, Type::Scalar) | (Type::Scalar, Type::Mat4) => Ok(Type::Mat4),
        (Type::Mat2, Type::Vec2) => Ok(Type::Vec2),
        #[cfg(feature = "3d")]
        (Type::Mat3, Type::Vec3) => Ok(Type::Vec3),
        #[cfg(feature = "4d")]
        (Type::Mat4, Type::Vec4) => Ok(Type::Vec4),
        (Type::Vec2, Type::Mat2) => Ok(Type::Vec2),
        #[cfg(feature = "3d")]
        (Type::Vec3, Type::Mat3) => Ok(Type::Vec3),
        #[cfg(feature = "4d")]
        (Type::Vec4, Type::Mat4) => Ok(Type::Vec4),
        (Type::Mat2, Type::Mat2) => Ok(Type::Mat2),
        #[cfg(feature = "3d")]
        (Type::Mat3, Type::Mat3) => Ok(Type::Mat3),
        #[cfg(feature = "4d")]
        (Type::Mat4, Type::Mat4) => Ok(Type::Mat4),
        _ => Err(OpenCLError::TypeMismatch {
            op: "*",
            left,
            right,
        }),
    }
}

fn emit_mul(
    left: &OpenCLExpr,
    right: &OpenCLExpr,
    _result_type: Type,
) -> Result<String, OpenCLError> {
    Ok(match (left.typ, right.typ) {
        (Type::Scalar, Type::Scalar) => format!("({} * {})", left.code, right.code),
        // OpenCL vectors support * with scalars directly
        (Type::Scalar, t) | (t, Type::Scalar) if !is_matrix_type(t) => {
            format!("({} * {})", left.code, right.code)
        }
        // Matrices need function calls
        (Type::Mat2, Type::Scalar) | (Type::Scalar, Type::Mat2) => {
            format!("mat2_scale({}, {})", left.code, right.code)
        }
        #[cfg(feature = "3d")]
        (Type::Mat3, Type::Scalar) | (Type::Scalar, Type::Mat3) => {
            format!("mat3_scale({}, {})", left.code, right.code)
        }
        #[cfg(feature = "4d")]
        (Type::Mat4, Type::Scalar) | (Type::Scalar, Type::Mat4) => {
            format!("mat4_scale({}, {})", left.code, right.code)
        }
        (Type::Mat2, Type::Vec2) => format!("mat2_mul_vec2({}, {})", left.code, right.code),
        #[cfg(feature = "3d")]
        (Type::Mat3, Type::Vec3) => format!("mat3_mul_vec3({}, {})", left.code, right.code),
        #[cfg(feature = "4d")]
        (Type::Mat4, Type::Vec4) => format!("mat4_mul_vec4({}, {})", left.code, right.code),
        (Type::Vec2, Type::Mat2) => format!("vec2_mul_mat2({}, {})", left.code, right.code),
        #[cfg(feature = "3d")]
        (Type::Vec3, Type::Mat3) => format!("vec3_mul_mat3({}, {})", left.code, right.code),
        #[cfg(feature = "4d")]
        (Type::Vec4, Type::Mat4) => format!("vec4_mul_mat4({}, {})", left.code, right.code),
        (Type::Mat2, Type::Mat2) => format!("mat2_mul({}, {})", left.code, right.code),
        #[cfg(feature = "3d")]
        (Type::Mat3, Type::Mat3) => format!("mat3_mul({}, {})", left.code, right.code),
        #[cfg(feature = "4d")]
        (Type::Mat4, Type::Mat4) => format!("mat4_mul({}, {})", left.code, right.code),
        _ => format!("({} * {})", left.code, right.code),
    })
}

fn emit_div(left: &OpenCLExpr, right: &OpenCLExpr) -> Result<String, OpenCLError> {
    Ok(match (left.typ, right.typ) {
        (Type::Scalar, Type::Scalar) => format!("({} / {})", left.code, right.code),
        // OpenCL vectors support / with scalars directly
        (t, Type::Scalar) if !is_matrix_type(t) => {
            format!("({} / {})", left.code, right.code)
        }
        _ => {
            return Err(OpenCLError::TypeMismatch {
                op: "/",
                left: left.typ,
                right: right.typ,
            });
        }
    })
}

fn emit_unaryop(op: UnaryOp, inner: OpenCLExpr) -> Result<OpenCLExpr, OpenCLError> {
    match op {
        UnaryOp::Neg => {
            // OpenCL vectors support unary minus
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
        UnaryOp::BitNot => {
            if inner.typ != Type::Scalar {
                return Err(OpenCLError::UnsupportedType(inner.typ));
            }
            Ok(OpenCLExpr {
                code: format!("(~(int){})", inner.code),
                typ: Type::Scalar,
            })
        }
    }
}

fn emit_function_call(name: &str, args: Vec<OpenCLExpr>) -> Result<OpenCLExpr, OpenCLError> {
    match name {
        "dot" => {
            if args.len() != 2 || args[0].typ != args[1].typ {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            // OpenCL has built-in dot()
            Ok(OpenCLExpr {
                code: format!("dot({}, {})", args[0].code, args[1].code),
                typ: Type::Scalar,
            })
        }

        #[cfg(feature = "3d")]
        "cross" => {
            if args.len() != 2 || args[0].typ != Type::Vec3 || args[1].typ != Type::Vec3 {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            // OpenCL has built-in cross()
            Ok(OpenCLExpr {
                code: format!("cross({}, {})", args[0].code, args[1].code),
                typ: Type::Vec3,
            })
        }

        "length" => {
            if args.len() != 1 {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            // OpenCL has built-in length()
            Ok(OpenCLExpr {
                code: format!("length({})", args[0].code),
                typ: Type::Scalar,
            })
        }

        "normalize" => {
            if args.len() != 1 {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            // OpenCL has built-in normalize()
            Ok(OpenCLExpr {
                code: format!("normalize({})", args[0].code),
                typ: args[0].typ,
            })
        }

        "distance" => {
            if args.len() != 2 || args[0].typ != args[1].typ {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            // OpenCL has built-in distance()
            Ok(OpenCLExpr {
                code: format!("distance({}, {})", args[0].code, args[1].code),
                typ: Type::Scalar,
            })
        }

        "lerp" | "mix" => {
            if args.len() != 3 || args[2].typ != Type::Scalar {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            // OpenCL uses mix()
            Ok(OpenCLExpr {
                code: format!("mix({}, {}, {})", args[0].code, args[1].code, args[2].code),
                typ: args[0].typ,
            })
        }

        "vec2" => {
            if args.len() != 2 || args.iter().any(|a| a.typ != Type::Scalar) {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            Ok(OpenCLExpr {
                code: format!("(float2)({}, {})", args[0].code, args[1].code),
                typ: Type::Vec2,
            })
        }

        #[cfg(feature = "3d")]
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

        #[cfg(feature = "4d")]
        "vec4" => {
            if args.len() != 4 || args.iter().any(|a| a.typ != Type::Scalar) {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            Ok(OpenCLExpr {
                code: format!(
                    "(float4)({}, {}, {}, {})",
                    args[0].code, args[1].code, args[2].code, args[3].code
                ),
                typ: Type::Vec4,
            })
        }

        "x" => {
            if args.len() != 1 {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            Ok(OpenCLExpr {
                code: format!("{}.x", args[0].code),
                typ: Type::Scalar,
            })
        }

        "y" => {
            if args.len() != 1 {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            Ok(OpenCLExpr {
                code: format!("{}.y", args[0].code),
                typ: Type::Scalar,
            })
        }

        #[cfg(feature = "3d")]
        "z" => {
            if args.len() != 1 {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            Ok(OpenCLExpr {
                code: format!("{}.z", args[0].code),
                typ: Type::Scalar,
            })
        }

        #[cfg(feature = "4d")]
        "w" => {
            if args.len() != 1 {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            Ok(OpenCLExpr {
                code: format!("{}.w", args[0].code),
                typ: Type::Scalar,
            })
        }

        // Scalar math functions - OpenCL built-ins
        "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "exp" | "log" | "sqrt" | "abs"
        | "floor" | "ceil" => {
            if args.len() != 1 || args[0].typ != Type::Scalar {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            let func = match name {
                "abs" => "fabs",
                other => other,
            };
            Ok(OpenCLExpr {
                code: format!("{}({})", func, args[0].code),
                typ: Type::Scalar,
            })
        }

        "min" | "max" => {
            if args.len() != 2 {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            if args[0].typ == Type::Scalar && args[1].typ == Type::Scalar {
                let func = if name == "min" { "fmin" } else { "fmax" };
                Ok(OpenCLExpr {
                    code: format!("{}({}, {})", func, args[0].code, args[1].code),
                    typ: Type::Scalar,
                })
            } else if args[0].typ == args[1].typ {
                // OpenCL has vectorized min/max
                Ok(OpenCLExpr {
                    code: format!("{}({}, {})", name, args[0].code, args[1].code),
                    typ: args[0].typ,
                })
            } else {
                Err(OpenCLError::TypeMismatch {
                    op: "min/max",
                    left: args[0].typ,
                    right: args[1].typ,
                })
            }
        }

        "clamp" => {
            if args.len() != 3 {
                return Err(OpenCLError::UnknownFunction(name.to_string()));
            }
            // OpenCL has built-in clamp()
            Ok(OpenCLExpr {
                code: format!(
                    "clamp({}, {}, {})",
                    args[0].code, args[1].code, args[2].code
                ),
                typ: args[0].typ,
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
    fn test_scalar_add() {
        let result = emit("a + b", &[("a", Type::Scalar), ("b", Type::Scalar)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert!(result.code.contains("+"));
    }

    #[test]
    fn test_vec2_add() {
        let result = emit("a + b", &[("a", Type::Vec2), ("b", Type::Vec2)]).unwrap();
        assert_eq!(result.typ, Type::Vec2);
        // OpenCL supports + directly
        assert!(result.code.contains("+"));
    }

    #[test]
    fn test_dot() {
        let result = emit("dot(a, b)", &[("a", Type::Vec2), ("b", Type::Vec2)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert!(result.code.contains("dot("));
    }

    #[test]
    fn test_length() {
        let result = emit("length(v)", &[("v", Type::Vec2)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert!(result.code.contains("length("));
    }

    #[test]
    fn test_normalize() {
        let result = emit("normalize(v)", &[("v", Type::Vec2)]).unwrap();
        assert_eq!(result.typ, Type::Vec2);
        assert!(result.code.contains("normalize("));
    }

    #[test]
    fn test_vec2_constructor() {
        let result = emit("vec2(x, y)", &[("x", Type::Scalar), ("y", Type::Scalar)]).unwrap();
        assert_eq!(result.typ, Type::Vec2);
        assert!(result.code.contains("(float2)"));
    }

    #[test]
    fn test_component_extraction() {
        let result = emit("x(v)", &[("v", Type::Vec2)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert!(result.code.contains(".x"));
    }

    #[test]
    fn test_lerp() {
        let result = emit(
            "lerp(a, b, t)",
            &[("a", Type::Vec2), ("b", Type::Vec2), ("t", Type::Scalar)],
        )
        .unwrap();
        assert_eq!(result.typ, Type::Vec2);
        assert!(result.code.contains("mix("));
    }

    #[test]
    fn test_clamp() {
        let result = emit(
            "clamp(v, lo, hi)",
            &[("v", Type::Vec2), ("lo", Type::Vec2), ("hi", Type::Vec2)],
        )
        .unwrap();
        assert_eq!(result.typ, Type::Vec2);
        assert!(result.code.contains("clamp("));
    }
}
