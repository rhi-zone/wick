//! WebAssembly bindings for Dew expression language.
//!
//! Provides parsing and code generation for use in web browsers.

use dew_core::Expr;
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// AST node representation for JavaScript.
#[derive(Serialize)]
pub struct JsAstNode {
    #[serde(rename = "type")]
    node_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<JsAstNode>>,
}

/// Parse result for JavaScript.
#[derive(Serialize)]
pub struct JsParseResult {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    ast: Option<JsAstNode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Code generation result for JavaScript.
#[derive(Serialize)]
pub struct JsCodeResult {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn code_ok(code: String) -> JsCodeResult {
    JsCodeResult {
        ok: true,
        code: Some(code),
        error: None,
    }
}

fn code_err(error: String) -> JsCodeResult {
    JsCodeResult {
        ok: false,
        code: None,
        error: Some(error),
    }
}

/// Parse a dew expression and return the AST as a JavaScript object.
#[wasm_bindgen]
pub fn parse(input: &str) -> JsValue {
    let result = match Expr::parse(input) {
        Ok(expr) => JsParseResult {
            ok: true,
            ast: Some(ast_to_js(expr.ast())),
            error: None,
        },
        Err(e) => JsParseResult {
            ok: false,
            ast: None,
            error: Some(e.to_string()),
        },
    };

    serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
}

/// Convert AST to JavaScript-friendly representation.
fn ast_to_js(ast: &dew_core::Ast) -> JsAstNode {
    use dew_core::Ast;

    match ast {
        Ast::Num(n) => JsAstNode {
            node_type: "Num".to_string(),
            value: Some(n.to_string()),
            children: None,
        },
        Ast::Var(name) => JsAstNode {
            node_type: "Var".to_string(),
            value: Some(name.clone()),
            children: None,
        },
        Ast::BinOp(op, left, right) => JsAstNode {
            node_type: "BinOp".to_string(),
            value: Some(format!("{:?}", op)),
            children: Some(vec![ast_to_js(left), ast_to_js(right)]),
        },
        Ast::UnaryOp(op, inner) => JsAstNode {
            node_type: "UnaryOp".to_string(),
            value: Some(format!("{:?}", op)),
            children: Some(vec![ast_to_js(inner)]),
        },
        Ast::Call(name, args) => JsAstNode {
            node_type: "Call".to_string(),
            value: Some(name.clone()),
            children: Some(args.iter().map(ast_to_js).collect()),
        },
        Ast::Compare(op, left, right) => JsAstNode {
            node_type: "Compare".to_string(),
            value: Some(format!("{:?}", op)),
            children: Some(vec![ast_to_js(left), ast_to_js(right)]),
        },
        Ast::And(left, right) => JsAstNode {
            node_type: "And".to_string(),
            value: None,
            children: Some(vec![ast_to_js(left), ast_to_js(right)]),
        },
        Ast::Or(left, right) => JsAstNode {
            node_type: "Or".to_string(),
            value: None,
            children: Some(vec![ast_to_js(left), ast_to_js(right)]),
        },
        Ast::If(cond, then_branch, else_branch) => JsAstNode {
            node_type: "If".to_string(),
            value: None,
            children: Some(vec![
                ast_to_js(cond),
                ast_to_js(then_branch),
                ast_to_js(else_branch),
            ]),
        },
        Ast::Let { name, value, body } => JsAstNode {
            node_type: "Let".to_string(),
            value: Some(name.clone()),
            children: Some(vec![ast_to_js(value), ast_to_js(body)]),
        },
    }
}

// =============================================================================
// Scalar backends (always available with "core" feature)
// =============================================================================

#[cfg(feature = "dew-scalar")]
mod scalar {
    use super::*;

    /// Generate WGSL code from a scalar expression.
    #[wasm_bindgen]
    pub fn emit_wgsl(input: &str) -> JsValue {
        use dew_scalar::wgsl;

        let result = match Expr::parse(input) {
            Ok(expr) => match wgsl::emit_wgsl(expr.ast()) {
                Ok(wgsl_expr) => code_ok(wgsl_expr.code),
                Err(e) => code_err(e.to_string()),
            },
            Err(e) => code_err(e.to_string()),
        };

        serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
    }

    /// Generate GLSL code from a scalar expression.
    #[wasm_bindgen]
    pub fn emit_glsl(input: &str) -> JsValue {
        use dew_scalar::glsl;

        let result = match Expr::parse(input) {
            Ok(expr) => match glsl::emit_glsl(expr.ast()) {
                Ok(glsl_expr) => code_ok(glsl_expr.code),
                Err(e) => code_err(e.to_string()),
            },
            Err(e) => code_err(e.to_string()),
        };

        serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
    }

    /// Generate Lua code from a scalar expression.
    #[wasm_bindgen]
    pub fn emit_lua(input: &str) -> JsValue {
        use dew_scalar::lua;

        let result = match Expr::parse(input) {
            Ok(expr) => match lua::emit_lua(expr.ast()) {
                Ok(lua_expr) => code_ok(lua_expr.code),
                Err(e) => code_err(e.to_string()),
            },
            Err(e) => code_err(e.to_string()),
        };

        serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
    }
}

// =============================================================================
// Linalg backends
// =============================================================================

#[cfg(feature = "dew-linalg")]
mod linalg {
    use super::*;
    use dew_linalg::Type;
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Deserialize)]
    struct VarTypes(HashMap<String, String>);

    fn parse_linalg_type(s: &str) -> Result<Type, String> {
        match s {
            "scalar" | "f32" | "float" => Ok(Type::Scalar),
            "vec2" => Ok(Type::Vec2),
            "vec3" => Ok(Type::Vec3),
            "vec4" => Ok(Type::Vec4),
            "mat2" => Ok(Type::Mat2),
            "mat3" => Ok(Type::Mat3),
            "mat4" => Ok(Type::Mat4),
            _ => Err(format!("unknown linalg type: {s}")),
        }
    }

    fn parse_var_types(js_types: JsValue) -> Result<HashMap<String, Type>, String> {
        let var_types: VarTypes =
            serde_wasm_bindgen::from_value(js_types).map_err(|e| e.to_string())?;

        var_types
            .0
            .into_iter()
            .map(|(name, type_str)| parse_linalg_type(&type_str).map(|t| (name, t)))
            .collect()
    }

    /// Generate WGSL code from a linalg expression.
    /// var_types: { "varName": "vec3", ... }
    #[wasm_bindgen]
    pub fn emit_wgsl_linalg(input: &str, var_types: JsValue) -> JsValue {
        use dew_linalg::wgsl;

        let result = match parse_var_types(var_types) {
            Ok(types) => match Expr::parse(input) {
                Ok(expr) => match wgsl::emit_wgsl(expr.ast(), &types) {
                    Ok(wgsl_expr) => code_ok(wgsl_expr.code),
                    Err(e) => code_err(e.to_string()),
                },
                Err(e) => code_err(e.to_string()),
            },
            Err(e) => code_err(e),
        };

        serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
    }

    /// Generate GLSL code from a linalg expression.
    /// var_types: { "varName": "vec3", ... }
    #[wasm_bindgen]
    pub fn emit_glsl_linalg(input: &str, var_types: JsValue) -> JsValue {
        use dew_linalg::glsl;

        let result = match parse_var_types(var_types) {
            Ok(types) => match Expr::parse(input) {
                Ok(expr) => match glsl::emit_glsl(expr.ast(), &types) {
                    Ok(glsl_expr) => code_ok(glsl_expr.code),
                    Err(e) => code_err(e.to_string()),
                },
                Err(e) => code_err(e.to_string()),
            },
            Err(e) => code_err(e),
        };

        serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
    }

    /// Generate Lua code from a linalg expression.
    /// var_types: { "varName": "vec3", ... }
    #[wasm_bindgen]
    pub fn emit_lua_linalg(input: &str, var_types: JsValue) -> JsValue {
        use dew_linalg::lua;

        let result = match parse_var_types(var_types) {
            Ok(types) => match Expr::parse(input) {
                Ok(expr) => match lua::emit_lua(expr.ast(), &types) {
                    Ok(lua_expr) => code_ok(lua_expr.code),
                    Err(e) => code_err(e.to_string()),
                },
                Err(e) => code_err(e.to_string()),
            },
            Err(e) => code_err(e),
        };

        serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
    }
}

// =============================================================================
// Complex backends
// =============================================================================

#[cfg(feature = "dew-complex")]
mod complex {
    use super::*;
    use dew_complex::Type;
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Deserialize)]
    struct VarTypes(HashMap<String, String>);

    fn parse_complex_type(s: &str) -> Result<Type, String> {
        match s {
            "scalar" | "f32" | "float" | "real" => Ok(Type::Scalar),
            "complex" | "c32" => Ok(Type::Complex),
            _ => Err(format!("unknown complex type: {s}")),
        }
    }

    fn parse_var_types(js_types: JsValue) -> Result<HashMap<String, Type>, String> {
        let var_types: VarTypes =
            serde_wasm_bindgen::from_value(js_types).map_err(|e| e.to_string())?;

        var_types
            .0
            .into_iter()
            .map(|(name, type_str)| parse_complex_type(&type_str).map(|t| (name, t)))
            .collect()
    }

    /// Generate WGSL code from a complex expression.
    /// var_types: { "z": "complex", "t": "scalar", ... }
    #[wasm_bindgen]
    pub fn emit_wgsl_complex(input: &str, var_types: JsValue) -> JsValue {
        use dew_complex::wgsl;

        let result = match parse_var_types(var_types) {
            Ok(types) => match Expr::parse(input) {
                Ok(expr) => match wgsl::emit_wgsl(expr.ast(), &types) {
                    Ok(wgsl_expr) => code_ok(wgsl_expr.code),
                    Err(e) => code_err(e.to_string()),
                },
                Err(e) => code_err(e.to_string()),
            },
            Err(e) => code_err(e),
        };

        serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
    }

    /// Generate GLSL code from a complex expression.
    /// var_types: { "z": "complex", "t": "scalar", ... }
    #[wasm_bindgen]
    pub fn emit_glsl_complex(input: &str, var_types: JsValue) -> JsValue {
        use dew_complex::glsl;

        let result = match parse_var_types(var_types) {
            Ok(types) => match Expr::parse(input) {
                Ok(expr) => match glsl::emit_glsl(expr.ast(), &types) {
                    Ok(glsl_expr) => code_ok(glsl_expr.code),
                    Err(e) => code_err(e.to_string()),
                },
                Err(e) => code_err(e.to_string()),
            },
            Err(e) => code_err(e),
        };

        serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
    }

    /// Generate Lua code from a complex expression.
    /// var_types: { "z": "complex", "t": "scalar", ... }
    #[wasm_bindgen]
    pub fn emit_lua_complex(input: &str, var_types: JsValue) -> JsValue {
        use dew_complex::lua;

        let result = match parse_var_types(var_types) {
            Ok(types) => match Expr::parse(input) {
                Ok(expr) => match lua::emit_lua(expr.ast(), &types) {
                    Ok(lua_expr) => code_ok(lua_expr.code),
                    Err(e) => code_err(e.to_string()),
                },
                Err(e) => code_err(e.to_string()),
            },
            Err(e) => code_err(e),
        };

        serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
    }
}

// =============================================================================
// Quaternion backends
// =============================================================================

#[cfg(feature = "dew-quaternion")]
mod quaternion {
    use super::*;
    use dew_quaternion::Type;
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Deserialize)]
    struct VarTypes(HashMap<String, String>);

    fn parse_quaternion_type(s: &str) -> Result<Type, String> {
        match s {
            "scalar" | "f32" | "float" => Ok(Type::Scalar),
            "vec3" => Ok(Type::Vec3),
            "quaternion" | "quat" => Ok(Type::Quaternion),
            _ => Err(format!("unknown quaternion type: {s}")),
        }
    }

    fn parse_var_types(js_types: JsValue) -> Result<HashMap<String, Type>, String> {
        let var_types: VarTypes =
            serde_wasm_bindgen::from_value(js_types).map_err(|e| e.to_string())?;

        var_types
            .0
            .into_iter()
            .map(|(name, type_str)| parse_quaternion_type(&type_str).map(|t| (name, t)))
            .collect()
    }

    /// Generate WGSL code from a quaternion expression.
    /// var_types: { "q": "quaternion", "v": "vec3", ... }
    #[wasm_bindgen]
    pub fn emit_wgsl_quaternion(input: &str, var_types: JsValue) -> JsValue {
        use dew_quaternion::wgsl;

        let result = match parse_var_types(var_types) {
            Ok(types) => match Expr::parse(input) {
                Ok(expr) => match wgsl::emit_wgsl(expr.ast(), &types) {
                    Ok(wgsl_expr) => code_ok(wgsl_expr.code),
                    Err(e) => code_err(e.to_string()),
                },
                Err(e) => code_err(e.to_string()),
            },
            Err(e) => code_err(e),
        };

        serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
    }

    /// Generate GLSL code from a quaternion expression.
    /// var_types: { "q": "quaternion", "v": "vec3", ... }
    #[wasm_bindgen]
    pub fn emit_glsl_quaternion(input: &str, var_types: JsValue) -> JsValue {
        use dew_quaternion::glsl;

        let result = match parse_var_types(var_types) {
            Ok(types) => match Expr::parse(input) {
                Ok(expr) => match glsl::emit_glsl(expr.ast(), &types) {
                    Ok(glsl_expr) => code_ok(glsl_expr.code),
                    Err(e) => code_err(e.to_string()),
                },
                Err(e) => code_err(e.to_string()),
            },
            Err(e) => code_err(e),
        };

        serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
    }

    /// Generate Lua code from a quaternion expression.
    /// var_types: { "q": "quaternion", "v": "vec3", ... }
    #[wasm_bindgen]
    pub fn emit_lua_quaternion(input: &str, var_types: JsValue) -> JsValue {
        use dew_quaternion::lua;

        let result = match parse_var_types(var_types) {
            Ok(types) => match Expr::parse(input) {
                Ok(expr) => match lua::emit_lua(expr.ast(), &types) {
                    Ok(lua_expr) => code_ok(lua_expr.code),
                    Err(e) => code_err(e.to_string()),
                },
                Err(e) => code_err(e.to_string()),
            },
            Err(e) => code_err(e),
        };

        serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
    }
}

// Tests are run via wasm-bindgen-test in the browser or node environment
// since they depend on JS interop that can't work on native targets
