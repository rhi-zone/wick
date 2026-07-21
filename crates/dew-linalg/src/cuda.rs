//! CUDA code generation for linear algebra expressions.
//!
//! Uses CUDA built-in vector types (float2, float3, float4) with external
//! helper functions for vector operations. CUDA vector types support
//! basic arithmetic via operator overloading when using CUDA's helper_math.h.
//!
//! # Vector Types
//!
//! | Type    | CUDA Type |
//! |---------|-----------|
//! | Vec2    | float2    |
//! | Vec3    | float3    |
//! | Vec4    | float4    |
//! | Mat2    | mat2_t    |
//! | Mat3    | mat3_t    |
//! | Mat4    | mat4_t    |
//!
//! # Required External Functions
//!
//! Vector operations (when not using helper_math.h):
//! - `dot2(float2, float2)`, `dot3(float3, float3)`, `dot4(float4, float4)`
//! - `cross(float3, float3)` -> float3
//! - `length2(float2)`, `length3(float3)`, `length4(float4)`
//! - `normalize2(float2)`, `normalize3(float3)`, `normalize4(float4)`
//!
//! Matrix operations:
//! - `mat2_mul`, `mat3_mul`, `mat4_mul` - matrix multiplication
//! - `mat2_mul_vec2`, `mat3_mul_vec3`, `mat4_mul_vec4` - matrix-vector multiplication

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
    /// Conditionals require scalar types.
    UnsupportedTypeForConditional(Type),
    /// Operation not supported for this type.
    UnsupportedOperation(&'static str),
    /// Feature not supported in expression-only codegen.
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
            CUDAError::UnsupportedOperation(op) => {
                write!(f, "unsupported operation: {op}")
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
        Type::Vec2 => "float2",
        Type::Mat2 => "mat2_t",
        #[cfg(feature = "3d")]
        Type::Vec3 => "float3",
        #[cfg(feature = "3d")]
        Type::Mat3 => "mat3_t",
        #[cfg(feature = "4d")]
        Type::Vec4 => "float4",
        #[cfg(feature = "4d")]
        Type::Mat4 => "mat4_t",
    }
}

/// Result of CUDA emission: code string and its type.
pub struct CUDAExpr {
    pub code: String,
    pub typ: Type,
}

/// Result of full CUDA emission with accumulated statements.
struct Emission {
    statements: Vec<String>,
    expr: String,
    typ: Type,
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
            if then_expr.typ != else_expr.typ {
                return Err(CUDAError::TypeMismatch {
                    op: "if/else",
                    left: then_expr.typ,
                    right: else_expr.typ,
                });
            }
            let cond_bool = cond::scalar_to_bool(&cond_expr.code);
            Ok(CUDAExpr {
                code: cond::emit_if(&cond_bool, &then_expr.code, &else_expr.code),
                typ: then_expr.typ,
            })
        }

        Ast::Let { .. } => {
            let emission = emit_full(ast, var_types)?;
            if !emission.statements.is_empty() {
                return Err(CUDAError::UnsupportedFeature(
                    "let bindings in expression context (use emit_cuda_fn)".to_string(),
                ));
            }
            Ok(CUDAExpr {
                code: emission.expr,
                typ: emission.typ,
            })
        }
    }
}

/// Emit CUDA with full statement support.
fn emit_full(ast: &Ast, var_types: &HashMap<String, Type>) -> Result<Emission, CUDAError> {
    match ast {
        Ast::Let { name, value, body } => {
            let value_emission = emit_full(value, var_types)?;
            let mut new_var_types = var_types.clone();
            new_var_types.insert(name.clone(), value_emission.typ);
            let body_emission = emit_full(body, &new_var_types)?;

            let mut statements = value_emission.statements;
            statements.push(format!(
                "{} {} = {};",
                type_to_cuda(value_emission.typ),
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
            let result = emit_cuda(ast, var_types)?;
            Ok(Emission {
                statements: vec![],
                expr: result.code,
                typ: result.typ,
            })
        }
    }
}

/// Emit a complete CUDA device function with let statement support.
pub fn emit_cuda_fn(
    name: &str,
    ast: &Ast,
    params: &[(&str, Type)],
    return_type: Type,
) -> Result<String, CUDAError> {
    let var_types: HashMap<String, Type> =
        params.iter().map(|(n, t)| (n.to_string(), *t)).collect();
    let emission = emit_full(ast, &var_types)?;

    let params_str = params
        .iter()
        .map(|(n, t)| format!("{} {}", type_to_cuda(*t), n))
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
        "__device__ {} {}({}) {{\n{}\n}}",
        type_to_cuda(return_type),
        name,
        params_str,
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

fn emit_binop(op: BinOp, left: CUDAExpr, right: CUDAExpr) -> Result<CUDAExpr, CUDAError> {
    let result_type = infer_binop_type(op, left.typ, right.typ)?;

    let code = match op {
        BinOp::Add => format!("({} + {})", left.code, right.code),
        BinOp::Sub => format!("({} - {})", left.code, right.code),
        BinOp::Mul => emit_mul(&left, &right)?,
        BinOp::Div => emit_div(&left, &right)?,
        BinOp::Pow => {
            if left.typ == Type::Scalar && right.typ == Type::Scalar {
                format!("powf({}, {})", left.code, right.code)
            } else {
                return Err(CUDAError::TypeMismatch {
                    op: "^",
                    left: left.typ,
                    right: right.typ,
                });
            }
        }
        BinOp::Rem => {
            if left.typ == Type::Scalar && right.typ == Type::Scalar {
                format!("fmodf({}, {})", left.code, right.code)
            } else {
                return Err(CUDAError::UnsupportedOperation("%"));
            }
        }
        BinOp::BitAnd | BinOp::BitOr | BinOp::Shl | BinOp::Shr => {
            return Err(CUDAError::UnsupportedOperation("bitwise"));
        }
    };

    Ok(CUDAExpr {
        code,
        typ: result_type,
    })
}

fn infer_binop_type(op: BinOp, left: Type, right: Type) -> Result<Type, CUDAError> {
    match op {
        BinOp::Add | BinOp::Sub => {
            if left == right {
                Ok(left)
            } else {
                Err(CUDAError::TypeMismatch {
                    op: "+/-",
                    left,
                    right,
                })
            }
        }
        BinOp::Mul => infer_mul_type(left, right),
        BinOp::Div => {
            if right == Type::Scalar {
                Ok(left)
            } else if left == Type::Scalar && right == Type::Scalar {
                Ok(Type::Scalar)
            } else {
                Err(CUDAError::TypeMismatch {
                    op: "/",
                    left,
                    right,
                })
            }
        }
        BinOp::Pow | BinOp::Rem => Ok(Type::Scalar),
        _ => Err(CUDAError::UnsupportedOperation("bitwise")),
    }
}

fn infer_mul_type(left: Type, right: Type) -> Result<Type, CUDAError> {
    match (left, right) {
        (Type::Scalar, Type::Scalar) => Ok(Type::Scalar),
        (Type::Scalar, t) | (t, Type::Scalar) => Ok(t),
        (Type::Vec2, Type::Vec2) => Ok(Type::Vec2), // Component-wise
        #[cfg(feature = "3d")]
        (Type::Vec3, Type::Vec3) => Ok(Type::Vec3),
        #[cfg(feature = "4d")]
        (Type::Vec4, Type::Vec4) => Ok(Type::Vec4),
        (Type::Mat2, Type::Vec2) => Ok(Type::Vec2),
        (Type::Vec2, Type::Mat2) => Ok(Type::Vec2),
        (Type::Mat2, Type::Mat2) => Ok(Type::Mat2),
        #[cfg(feature = "3d")]
        (Type::Mat3, Type::Vec3) => Ok(Type::Vec3),
        #[cfg(feature = "3d")]
        (Type::Vec3, Type::Mat3) => Ok(Type::Vec3),
        #[cfg(feature = "3d")]
        (Type::Mat3, Type::Mat3) => Ok(Type::Mat3),
        #[cfg(feature = "4d")]
        (Type::Mat4, Type::Vec4) => Ok(Type::Vec4),
        #[cfg(feature = "4d")]
        (Type::Vec4, Type::Mat4) => Ok(Type::Vec4),
        #[cfg(feature = "4d")]
        (Type::Mat4, Type::Mat4) => Ok(Type::Mat4),
        _ => Err(CUDAError::TypeMismatch {
            op: "*",
            left,
            right,
        }),
    }
}

fn emit_mul(left: &CUDAExpr, right: &CUDAExpr) -> Result<String, CUDAError> {
    Ok(match (left.typ, right.typ) {
        (Type::Scalar, Type::Scalar) => format!("({} * {})", left.code, right.code),
        // Scalar * vector or vector * scalar
        (Type::Scalar, _) | (_, Type::Scalar)
            if !is_matrix_type(left.typ) && !is_matrix_type(right.typ) =>
        {
            format!("({} * {})", left.code, right.code)
        }
        // Vector component-wise multiplication
        (Type::Vec2, Type::Vec2) => format!("({} * {})", left.code, right.code),
        #[cfg(feature = "3d")]
        (Type::Vec3, Type::Vec3) => format!("({} * {})", left.code, right.code),
        #[cfg(feature = "4d")]
        (Type::Vec4, Type::Vec4) => format!("({} * {})", left.code, right.code),
        // Matrix * scalar
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
        // Matrix * vector
        (Type::Mat2, Type::Vec2) => format!("mat2_mul_vec2({}, {})", left.code, right.code),
        #[cfg(feature = "3d")]
        (Type::Mat3, Type::Vec3) => format!("mat3_mul_vec3({}, {})", left.code, right.code),
        #[cfg(feature = "4d")]
        (Type::Mat4, Type::Vec4) => format!("mat4_mul_vec4({}, {})", left.code, right.code),
        // Vector * matrix
        (Type::Vec2, Type::Mat2) => format!("vec2_mul_mat2({}, {})", left.code, right.code),
        #[cfg(feature = "3d")]
        (Type::Vec3, Type::Mat3) => format!("vec3_mul_mat3({}, {})", left.code, right.code),
        #[cfg(feature = "4d")]
        (Type::Vec4, Type::Mat4) => format!("vec4_mul_mat4({}, {})", left.code, right.code),
        // Matrix * matrix
        (Type::Mat2, Type::Mat2) => format!("mat2_mul({}, {})", left.code, right.code),
        #[cfg(feature = "3d")]
        (Type::Mat3, Type::Mat3) => format!("mat3_mul({}, {})", left.code, right.code),
        #[cfg(feature = "4d")]
        (Type::Mat4, Type::Mat4) => format!("mat4_mul({}, {})", left.code, right.code),
        _ => format!("({} * {})", left.code, right.code),
    })
}

fn emit_div(left: &CUDAExpr, right: &CUDAExpr) -> Result<String, CUDAError> {
    Ok(match (left.typ, right.typ) {
        (Type::Scalar, Type::Scalar) => format!("({} / {})", left.code, right.code),
        (_, Type::Scalar) if !is_matrix_type(left.typ) => {
            format!("({} / {})", left.code, right.code)
        }
        _ => {
            return Err(CUDAError::TypeMismatch {
                op: "/",
                left: left.typ,
                right: right.typ,
            });
        }
    })
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
        "vec2" => {
            if args.len() != 2 || args.iter().any(|a| a.typ != Type::Scalar) {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            Ok(CUDAExpr {
                code: format!("make_float2({}, {})", args[0].code, args[1].code),
                typ: Type::Vec2,
            })
        }

        #[cfg(feature = "3d")]
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

        #[cfg(feature = "4d")]
        "vec4" => {
            if args.len() != 4 || args.iter().any(|a| a.typ != Type::Scalar) {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            Ok(CUDAExpr {
                code: format!(
                    "make_float4({}, {}, {}, {})",
                    args[0].code, args[1].code, args[2].code, args[3].code
                ),
                typ: Type::Vec4,
            })
        }

        "dot" => {
            if args.len() != 2 || args[0].typ != args[1].typ {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            let func = match args[0].typ {
                Type::Vec2 => "dot2",
                #[cfg(feature = "3d")]
                Type::Vec3 => "dot3",
                #[cfg(feature = "4d")]
                Type::Vec4 => "dot4",
                _ => return Err(CUDAError::UnknownFunction(name.to_string())),
            };
            Ok(CUDAExpr {
                code: format!("{}({}, {})", func, args[0].code, args[1].code),
                typ: Type::Scalar,
            })
        }

        #[cfg(feature = "3d")]
        "cross" => {
            if args.len() != 2 || args[0].typ != Type::Vec3 || args[1].typ != Type::Vec3 {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            Ok(CUDAExpr {
                code: format!("cross({}, {})", args[0].code, args[1].code),
                typ: Type::Vec3,
            })
        }

        "length" => {
            if args.len() != 1 {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            let func = match args[0].typ {
                Type::Scalar => {
                    return Ok(CUDAExpr {
                        code: format!("fabsf({})", args[0].code),
                        typ: Type::Scalar,
                    });
                }
                Type::Vec2 => "length2",
                #[cfg(feature = "3d")]
                Type::Vec3 => "length3",
                #[cfg(feature = "4d")]
                Type::Vec4 => "length4",
                _ => return Err(CUDAError::UnknownFunction(name.to_string())),
            };
            Ok(CUDAExpr {
                code: format!("{}({})", func, args[0].code),
                typ: Type::Scalar,
            })
        }

        "normalize" => {
            if args.len() != 1 {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            let func = match args[0].typ {
                Type::Vec2 => "normalize2",
                #[cfg(feature = "3d")]
                Type::Vec3 => "normalize3",
                #[cfg(feature = "4d")]
                Type::Vec4 => "normalize4",
                _ => return Err(CUDAError::UnknownFunction(name.to_string())),
            };
            Ok(CUDAExpr {
                code: format!("{}({})", func, args[0].code),
                typ: args[0].typ,
            })
        }

        "distance" => {
            if args.len() != 2 || args[0].typ != args[1].typ {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            let func = match args[0].typ {
                Type::Vec2 => "distance2",
                #[cfg(feature = "3d")]
                Type::Vec3 => "distance3",
                #[cfg(feature = "4d")]
                Type::Vec4 => "distance4",
                _ => return Err(CUDAError::UnknownFunction(name.to_string())),
            };
            Ok(CUDAExpr {
                code: format!("{}({}, {})", func, args[0].code, args[1].code),
                typ: Type::Scalar,
            })
        }

        "lerp" | "mix" => {
            if args.len() != 3 || args[0].typ != args[1].typ || args[2].typ != Type::Scalar {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            // CUDA has lerp for float, use formula for vectors
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

        "clamp" => {
            if args.len() != 3 {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(CUDAExpr {
                    code: format!(
                        "fminf(fmaxf({}, {}), {})",
                        args[0].code, args[1].code, args[2].code
                    ),
                    typ: Type::Scalar,
                }),
                Type::Vec2 => Ok(CUDAExpr {
                    code: format!(
                        "clamp2({}, {}, {})",
                        args[0].code, args[1].code, args[2].code
                    ),
                    typ: Type::Vec2,
                }),
                #[cfg(feature = "3d")]
                Type::Vec3 => Ok(CUDAExpr {
                    code: format!(
                        "clamp3({}, {}, {})",
                        args[0].code, args[1].code, args[2].code
                    ),
                    typ: Type::Vec3,
                }),
                #[cfg(feature = "4d")]
                Type::Vec4 => Ok(CUDAExpr {
                    code: format!(
                        "clamp4({}, {}, {})",
                        args[0].code, args[1].code, args[2].code
                    ),
                    typ: Type::Vec4,
                }),
                _ => Err(CUDAError::UnknownFunction(name.to_string())),
            }
        }

        "min" => {
            if args.len() != 2 || args[0].typ != args[1].typ {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(CUDAExpr {
                    code: format!("fminf({}, {})", args[0].code, args[1].code),
                    typ: Type::Scalar,
                }),
                Type::Vec2 => Ok(CUDAExpr {
                    code: format!("min2({}, {})", args[0].code, args[1].code),
                    typ: Type::Vec2,
                }),
                #[cfg(feature = "3d")]
                Type::Vec3 => Ok(CUDAExpr {
                    code: format!("min3({}, {})", args[0].code, args[1].code),
                    typ: Type::Vec3,
                }),
                #[cfg(feature = "4d")]
                Type::Vec4 => Ok(CUDAExpr {
                    code: format!("min4({}, {})", args[0].code, args[1].code),
                    typ: Type::Vec4,
                }),
                _ => Err(CUDAError::UnknownFunction(name.to_string())),
            }
        }

        "max" => {
            if args.len() != 2 || args[0].typ != args[1].typ {
                return Err(CUDAError::UnknownFunction(name.to_string()));
            }
            match args[0].typ {
                Type::Scalar => Ok(CUDAExpr {
                    code: format!("fmaxf({}, {})", args[0].code, args[1].code),
                    typ: Type::Scalar,
                }),
                Type::Vec2 => Ok(CUDAExpr {
                    code: format!("max2({}, {})", args[0].code, args[1].code),
                    typ: Type::Vec2,
                }),
                #[cfg(feature = "3d")]
                Type::Vec3 => Ok(CUDAExpr {
                    code: format!("max3({}, {})", args[0].code, args[1].code),
                    typ: Type::Vec3,
                }),
                #[cfg(feature = "4d")]
                Type::Vec4 => Ok(CUDAExpr {
                    code: format!("max4({}, {})", args[0].code, args[1].code),
                    typ: Type::Vec4,
                }),
                _ => Err(CUDAError::UnknownFunction(name.to_string())),
            }
        }

        // Component extraction
        "x" => extract_component(&args, 0, "x"),
        "y" => extract_component(&args, 1, "y"),
        #[cfg(feature = "3d")]
        "z" => extract_component(&args, 2, "z"),
        #[cfg(feature = "4d")]
        "w" => extract_component(&args, 3, "w"),

        _ => Err(CUDAError::UnknownFunction(name.to_string())),
    }
}

fn extract_component(
    args: &[CUDAExpr],
    _idx: usize,
    component: &str,
) -> Result<CUDAExpr, CUDAError> {
    if args.len() != 1 {
        return Err(CUDAError::UnknownFunction(component.to_string()));
    }
    Ok(CUDAExpr {
        code: format!("{}.{}", args[0].code, component),
        typ: Type::Scalar,
    })
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
    fn test_scalar_add() {
        let result = emit("a + b", &[("a", Type::Scalar), ("b", Type::Scalar)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert_eq!(result.code, "(a + b)");
    }

    #[test]
    fn test_vec2_add() {
        let result = emit("a + b", &[("a", Type::Vec2), ("b", Type::Vec2)]).unwrap();
        assert_eq!(result.typ, Type::Vec2);
        assert_eq!(result.code, "(a + b)");
    }

    #[test]
    fn test_vec2_constructor() {
        let result = emit("vec2(x, y)", &[("x", Type::Scalar), ("y", Type::Scalar)]).unwrap();
        assert_eq!(result.typ, Type::Vec2);
        assert!(result.code.contains("make_float2"));
    }

    #[test]
    fn test_dot() {
        let result = emit("dot(a, b)", &[("a", Type::Vec2), ("b", Type::Vec2)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert!(result.code.contains("dot2"));
    }

    #[test]
    fn test_length() {
        let result = emit("length(v)", &[("v", Type::Vec2)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert!(result.code.contains("length2"));
    }

    #[test]
    fn test_normalize() {
        let result = emit("normalize(v)", &[("v", Type::Vec2)]).unwrap();
        assert_eq!(result.typ, Type::Vec2);
        assert!(result.code.contains("normalize2"));
    }

    #[test]
    fn test_component_extraction() {
        let result = emit("x(v)", &[("v", Type::Vec2)]).unwrap();
        assert_eq!(result.typ, Type::Scalar);
        assert_eq!(result.code, "v.x");
    }

    #[test]
    fn test_emit_cuda_fn() {
        let expr = Expr::parse("a + b").unwrap();
        let code = emit_cuda_fn(
            "add_vecs",
            expr.ast(),
            &[("a", Type::Vec2), ("b", Type::Vec2)],
            Type::Vec2,
        )
        .unwrap();
        assert!(code.contains("__device__"));
        assert!(code.contains("float2 add_vecs"));
    }

    #[test]
    fn test_lerp() {
        let result = emit(
            "lerp(a, b, t)",
            &[
                ("a", Type::Scalar),
                ("b", Type::Scalar),
                ("t", Type::Scalar),
            ],
        )
        .unwrap();
        assert!(result.code.contains("lerp"));
    }
}
