//! C code generation for linalg expressions.
//!
//! Emits C code assuming vec2/vec3/vec4/mat2/mat3/mat4 types exist.
//! Uses function-based operations (e.g., vec2_add, vec2_dot, mat2_mul_vec2).
//!
//! The generated code assumes these types and functions are defined elsewhere:
//! - Types: vec2, vec3, vec4, mat2, mat3, mat4
//! - Constructors: vec2_new(x,y), vec3_new(x,y,z), etc.
//! - Operations: vec2_add, vec2_sub, vec2_scale, vec2_dot, vec2_length, etc.

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
    UnsupportedType(Type),
    UnsupportedTypeForConditional(Type),
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
            CError::UnsupportedType(t) => write!(f, "unsupported type: {t}"),
            CError::UnsupportedTypeForConditional(t) => {
                write!(f, "conditionals require scalar type, got {t}")
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
        Type::Vec2 => "vec2",
        #[cfg(feature = "3d")]
        Type::Vec3 => "vec3",
        #[cfg(feature = "4d")]
        Type::Vec4 => "vec4",
        Type::Mat2 => "mat2",
        #[cfg(feature = "3d")]
        Type::Mat3 => "mat3",
        #[cfg(feature = "4d")]
        Type::Mat4 => "mat4",
    }
}

/// Result of C emission: code string and its type.
pub struct CExpr {
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

/// Generate a complete C function.
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

fn type_prefix(t: Type) -> &'static str {
    match t {
        Type::Scalar => "scalar",
        Type::Vec2 => "vec2",
        #[cfg(feature = "3d")]
        Type::Vec3 => "vec3",
        #[cfg(feature = "4d")]
        Type::Vec4 => "vec4",
        Type::Mat2 => "mat2",
        #[cfg(feature = "3d")]
        Type::Mat3 => "mat3",
        #[cfg(feature = "4d")]
        Type::Mat4 => "mat4",
    }
}

fn emit_binop(op: BinOp, left: CExpr, right: CExpr) -> Result<CExpr, CError> {
    let result_type = infer_binop_type(op, left.typ, right.typ)?;

    let code = match op {
        BinOp::Add => emit_add(&left, &right, result_type),
        BinOp::Sub => emit_sub(&left, &right, result_type),
        BinOp::Mul => emit_mul(&left, &right, result_type),
        BinOp::Div => emit_div(&left, &right, result_type)?,
        BinOp::Pow => {
            if left.typ == Type::Scalar && right.typ == Type::Scalar {
                format!("powf({}, {})", left.code, right.code)
            } else {
                return Err(CError::TypeMismatch {
                    op: "^",
                    left: left.typ,
                    right: right.typ,
                });
            }
        }
        BinOp::Rem | BinOp::BitAnd | BinOp::BitOr | BinOp::Shl | BinOp::Shr => {
            if left.typ == Type::Scalar && right.typ == Type::Scalar {
                let op_str = match op {
                    BinOp::Rem => {
                        return Ok(CExpr {
                            code: format!("fmodf({}, {})", left.code, right.code),
                            typ: Type::Scalar,
                        });
                    }
                    BinOp::BitAnd => "&",
                    BinOp::BitOr => "|",
                    BinOp::Shl => "<<",
                    BinOp::Shr => ">>",
                    _ => unreachable!(),
                };
                format!("((int){} {} (int){})", left.code, op_str, right.code)
            } else {
                return Err(CError::TypeMismatch {
                    op: match op {
                        BinOp::Rem => "%",
                        BinOp::BitAnd => "&",
                        BinOp::BitOr => "|",
                        BinOp::Shl => "<<",
                        BinOp::Shr => ">>",
                        _ => unreachable!(),
                    },
                    left: left.typ,
                    right: right.typ,
                });
            }
        }
    };

    Ok(CExpr {
        code,
        typ: result_type,
    })
}

fn infer_binop_type(op: BinOp, left: Type, right: Type) -> Result<Type, CError> {
    match op {
        BinOp::Add | BinOp::Sub => {
            if left == right {
                Ok(left)
            } else {
                Err(CError::TypeMismatch {
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
            _ => Err(CError::TypeMismatch {
                op: "/",
                left,
                right,
            }),
        },
        _ => {
            if left == Type::Scalar && right == Type::Scalar {
                Ok(Type::Scalar)
            } else {
                Err(CError::TypeMismatch {
                    op: "binop",
                    left,
                    right,
                })
            }
        }
    }
}

fn infer_mul_type(left: Type, right: Type) -> Result<Type, CError> {
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
        _ => Err(CError::TypeMismatch {
            op: "*",
            left,
            right,
        }),
    }
}

fn emit_add(left: &CExpr, right: &CExpr, result_type: Type) -> String {
    if result_type == Type::Scalar {
        format!("({} + {})", left.code, right.code)
    } else {
        format!(
            "{}_add({}, {})",
            type_prefix(result_type),
            left.code,
            right.code
        )
    }
}

fn emit_sub(left: &CExpr, right: &CExpr, result_type: Type) -> String {
    if result_type == Type::Scalar {
        format!("({} - {})", left.code, right.code)
    } else {
        format!(
            "{}_sub({}, {})",
            type_prefix(result_type),
            left.code,
            right.code
        )
    }
}

fn emit_mul(left: &CExpr, right: &CExpr, result_type: Type) -> String {
    match (left.typ, right.typ) {
        (Type::Scalar, Type::Scalar) => format!("({} * {})", left.code, right.code),
        (Type::Scalar, t) | (t, Type::Scalar) if t != Type::Scalar => {
            let (scalar, vec) = if left.typ == Type::Scalar {
                (&left.code, &right.code)
            } else {
                (&right.code, &left.code)
            };
            format!("{}_scale({}, {})", type_prefix(result_type), vec, scalar)
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
    }
}

fn emit_div(left: &CExpr, right: &CExpr, _result_type: Type) -> Result<String, CError> {
    match (left.typ, right.typ) {
        (Type::Scalar, Type::Scalar) => Ok(format!("({} / {})", left.code, right.code)),
        (t, Type::Scalar) if t != Type::Scalar => Ok(format!(
            "{}_scale({}, 1.0f / {})",
            type_prefix(t),
            left.code,
            right.code
        )),
        _ => Err(CError::TypeMismatch {
            op: "/",
            left: left.typ,
            right: right.typ,
        }),
    }
}

fn emit_unaryop(op: UnaryOp, inner: CExpr) -> Result<CExpr, CError> {
    match op {
        UnaryOp::Neg => {
            let code = if inner.typ == Type::Scalar {
                format!("(-{})", inner.code)
            } else {
                format!("{}_neg({})", type_prefix(inner.typ), inner.code)
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
        UnaryOp::BitNot => {
            if inner.typ != Type::Scalar {
                return Err(CError::UnsupportedType(inner.typ));
            }
            Ok(CExpr {
                code: format!("(~(int){})", inner.code),
                typ: Type::Scalar,
            })
        }
    }
}

fn emit_function_call(name: &str, args: Vec<CExpr>) -> Result<CExpr, CError> {
    match name {
        "dot" => {
            if args.len() != 2 || args[0].typ != args[1].typ {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!(
                    "{}_dot({}, {})",
                    type_prefix(args[0].typ),
                    args[0].code,
                    args[1].code
                ),
                typ: Type::Scalar,
            })
        }

        #[cfg(feature = "3d")]
        "cross" => {
            if args.len() != 2 || args[0].typ != Type::Vec3 || args[1].typ != Type::Vec3 {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!("vec3_cross({}, {})", args[0].code, args[1].code),
                typ: Type::Vec3,
            })
        }

        "length" => {
            if args.len() != 1 {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!("{}_length({})", type_prefix(args[0].typ), args[0].code),
                typ: Type::Scalar,
            })
        }

        "normalize" => {
            if args.len() != 1 {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!("{}_normalize({})", type_prefix(args[0].typ), args[0].code),
                typ: args[0].typ,
            })
        }

        "distance" => {
            if args.len() != 2 || args[0].typ != args[1].typ {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!(
                    "{}_distance({}, {})",
                    type_prefix(args[0].typ),
                    args[0].code,
                    args[1].code
                ),
                typ: Type::Scalar,
            })
        }

        "lerp" | "mix" => {
            if args.len() != 3 || args[2].typ != Type::Scalar {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!(
                    "{}_lerp({}, {}, {})",
                    type_prefix(args[0].typ),
                    args[0].code,
                    args[1].code,
                    args[2].code
                ),
                typ: args[0].typ,
            })
        }

        "vec2" => {
            if args.len() != 2 || args.iter().any(|a| a.typ != Type::Scalar) {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!("vec2_new({}, {})", args[0].code, args[1].code),
                typ: Type::Vec2,
            })
        }

        #[cfg(feature = "3d")]
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

        #[cfg(feature = "4d")]
        "vec4" => {
            if args.len() != 4 || args.iter().any(|a| a.typ != Type::Scalar) {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!(
                    "vec4_new({}, {}, {}, {})",
                    args[0].code, args[1].code, args[2].code, args[3].code
                ),
                typ: Type::Vec4,
            })
        }

        "x" => {
            if args.len() != 1 {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!("{}.x", args[0].code),
                typ: Type::Scalar,
            })
        }

        "y" => {
            if args.len() != 1 {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!("{}.y", args[0].code),
                typ: Type::Scalar,
            })
        }

        #[cfg(feature = "3d")]
        "z" => {
            if args.len() != 1 {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!("{}.z", args[0].code),
                typ: Type::Scalar,
            })
        }

        #[cfg(feature = "4d")]
        "w" => {
            if args.len() != 1 {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            Ok(CExpr {
                code: format!("{}.w", args[0].code),
                typ: Type::Scalar,
            })
        }

        // Scalar math functions
        "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "exp" | "log" | "sqrt" | "abs"
        | "floor" | "ceil" => {
            if args.len() != 1 || args[0].typ != Type::Scalar {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            let func = match name {
                "sin" => "sinf",
                "cos" => "cosf",
                "tan" => "tanf",
                "asin" => "asinf",
                "acos" => "acosf",
                "atan" => "atanf",
                "exp" => "expf",
                "log" => "logf",
                "sqrt" => "sqrtf",
                "abs" => "fabsf",
                "floor" => "floorf",
                "ceil" => "ceilf",
                _ => unreachable!(),
            };
            Ok(CExpr {
                code: format!("{}({})", func, args[0].code),
                typ: Type::Scalar,
            })
        }

        "min" | "max" => {
            if args.len() != 2 {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            if args[0].typ == Type::Scalar && args[1].typ == Type::Scalar {
                let func = if name == "min" { "fminf" } else { "fmaxf" };
                Ok(CExpr {
                    code: format!("{}({}, {})", func, args[0].code, args[1].code),
                    typ: Type::Scalar,
                })
            } else if args[0].typ == args[1].typ {
                Ok(CExpr {
                    code: format!(
                        "{}_{name}({}, {})",
                        type_prefix(args[0].typ),
                        args[0].code,
                        args[1].code
                    ),
                    typ: args[0].typ,
                })
            } else {
                Err(CError::TypeMismatch {
                    op: "min/max",
                    left: args[0].typ,
                    right: args[1].typ,
                })
            }
        }

        "clamp" => {
            if args.len() != 3 {
                return Err(CError::UnknownFunction(name.to_string()));
            }
            if args[0].typ == Type::Scalar {
                Ok(CExpr {
                    code: format!(
                        "fminf(fmaxf({}, {}), {})",
                        args[0].code, args[1].code, args[2].code
                    ),
                    typ: Type::Scalar,
                })
            } else {
                Ok(CExpr {
                    code: format!(
                        "{}_clamp({}, {}, {})",
                        type_prefix(args[0].typ),
                        args[0].code,
                        args[1].code,
                        args[2].code
                    ),
                    typ: args[0].typ,
                })
            }
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
    fn test_scalar_add() {
        let result = emit("a + b", &[("a", Type::Scalar), ("b", Type::Scalar)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert!(result.code.contains("+"));
    }

    #[test]
    fn test_vec2_add() {
        let result = emit("a + b", &[("a", Type::Vec2), ("b", Type::Vec2)]).unwrap();
        assert_eq!(result.typ, Type::Vec2);
        assert!(result.code.contains("vec2_add"));
    }

    #[test]
    fn test_mat_vec_mul() {
        let result = emit("m * v", &[("m", Type::Mat2), ("v", Type::Vec2)]).unwrap();
        assert_eq!(result.typ, Type::Vec2);
        assert!(result.code.contains("mat2_mul_vec2"));
    }

    #[test]
    fn test_dot() {
        let result = emit("dot(a, b)", &[("a", Type::Vec2), ("b", Type::Vec2)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert!(result.code.contains("vec2_dot"));
    }

    #[test]
    fn test_length() {
        let result = emit("length(v)", &[("v", Type::Vec2)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert!(result.code.contains("vec2_length"));
    }

    #[test]
    fn test_normalize() {
        let result = emit("normalize(v)", &[("v", Type::Vec2)]).unwrap();
        assert_eq!(result.typ, Type::Vec2);
        assert!(result.code.contains("vec2_normalize"));
    }

    #[test]
    fn test_vec2_constructor() {
        let result = emit("vec2(x, y)", &[("x", Type::Scalar), ("y", Type::Scalar)]).unwrap();
        assert_eq!(result.typ, Type::Vec2);
        assert!(result.code.contains("vec2_new"));
    }

    #[test]
    fn test_component_extraction() {
        let result = emit("x(v)", &[("v", Type::Vec2)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert!(result.code.contains(".x"));
    }

    #[test]
    fn test_type_mismatch() {
        let result = emit("a + b", &[("a", Type::Scalar), ("b", Type::Vec2)]);
        assert!(matches!(result, Err(CError::TypeMismatch { .. })));
    }
}
