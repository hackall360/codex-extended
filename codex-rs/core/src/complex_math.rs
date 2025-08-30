use std::collections::BTreeMap;
use std::sync::LazyLock;

use crate::openai_tools::{JsonSchema, OpenAiTool, ResponsesApiTool};
use codex_protocol::models::{FunctionCallOutputPayload, ResponseInputItem};
use num_complex::Complex64;
use serde::Deserialize;

/// Built-in replacements for the former `complex_math` plugin.

pub(crate) static CALCULATE_TOOL: LazyLock<OpenAiTool> = LazyLock::new(|| {
    let mut props = BTreeMap::new();
    props.insert(
        "expr".to_string(),
        JsonSchema::String {
            description: Some("Expression to evaluate".to_string()),
        },
    );
    OpenAiTool::Function(ResponsesApiTool {
        name: "calculate".to_string(),
        description:
            "Evaluate a mathematical expression supporting common operators and functions."
                .to_string(),
        strict: true,
        parameters: JsonSchema::Object {
            properties: props,
            required: Some(vec!["expr".to_string()]),
            additional_properties: Some(false),
        },
    })
});

pub(crate) static QUADRATIC_SOLVE_TOOL: LazyLock<OpenAiTool> = LazyLock::new(|| {
    let mut props = BTreeMap::new();
    props.insert(
        "a".to_string(),
        JsonSchema::Number {
            description: Some("Coefficient for x^2".to_string()),
        },
    );
    props.insert(
        "b".to_string(),
        JsonSchema::Number {
            description: Some("Coefficient for x".to_string()),
        },
    );
    props.insert(
        "c".to_string(),
        JsonSchema::Number {
            description: Some("Constant term".to_string()),
        },
    );
    OpenAiTool::Function(ResponsesApiTool {
        name: "quadratic_solve".to_string(),
        description: "Solve ax^2 + bx + c = 0 for real or complex roots.".to_string(),
        strict: true,
        parameters: JsonSchema::Object {
            properties: props,
            required: Some(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
            additional_properties: Some(false),
        },
    })
});

pub(crate) static MATRIX_DET_TOOL: LazyLock<OpenAiTool> = LazyLock::new(|| {
    let mut props = BTreeMap::new();
    props.insert(
        "matrix".to_string(),
        JsonSchema::Array {
            items: Box::new(JsonSchema::Array {
                items: Box::new(JsonSchema::Number { description: None }),
                description: None,
            }),
            description: Some("2x2 or 3x3 matrix".to_string()),
        },
    );
    OpenAiTool::Function(ResponsesApiTool {
        name: "matrix_det".to_string(),
        description: "Compute the determinant of a 2x2 or 3x3 matrix.".to_string(),
        strict: true,
        parameters: JsonSchema::Object {
            properties: props,
            required: Some(vec!["matrix".to_string()]),
            additional_properties: Some(false),
        },
    })
});

#[derive(Deserialize)]
struct CalculateArgs {
    expr: String,
}

#[derive(Deserialize)]
struct QuadraticArgs {
    a: f64,
    b: f64,
    c: f64,
}

#[derive(Deserialize)]
struct MatrixArgs {
    matrix: Vec<Vec<f64>>,
}

pub(crate) fn handle_calculate(arguments: String, call_id: String) -> ResponseInputItem {
    match serde_json::from_str::<CalculateArgs>(&arguments) {
        Ok(args) => match meval::eval_str(&args.expr) {
            Ok(val) => ResponseInputItem::FunctionCallOutput {
                call_id,
                output: FunctionCallOutputPayload {
                    content: format!("result: {val}"),
                    success: Some(true),
                },
            },
            Err(e) => ResponseInputItem::FunctionCallOutput {
                call_id,
                output: FunctionCallOutputPayload {
                    content: format!("error: {e}"),
                    success: Some(false),
                },
            },
        },
        Err(e) => ResponseInputItem::FunctionCallOutput {
            call_id,
            output: FunctionCallOutputPayload {
                content: format!("failed to parse function arguments: {e}"),
                success: Some(false),
            },
        },
    }
}

pub(crate) fn handle_quadratic_solve(arguments: String, call_id: String) -> ResponseInputItem {
    match serde_json::from_str::<QuadraticArgs>(&arguments) {
        Ok(args) => {
            if args.a == 0.0 {
                return ResponseInputItem::FunctionCallOutput {
                    call_id,
                    output: FunctionCallOutputPayload {
                        content: "error: a cannot be 0".to_string(),
                        success: Some(false),
                    },
                };
            }
            let disc = args.b * args.b - 4.0 * args.a * args.c;
            let two_a = 2.0 * args.a;
            let roots: [Complex64; 2] = if disc >= 0.0 {
                let rdisc = disc.sqrt();
                [
                    Complex64::new((-args.b + rdisc) / two_a, 0.0),
                    Complex64::new((-args.b - rdisc) / two_a, 0.0),
                ]
            } else {
                let rdisc = (-disc).sqrt();
                [
                    Complex64::new(-args.b / two_a, rdisc / two_a),
                    Complex64::new(-args.b / two_a, -rdisc / two_a),
                ]
            };
            ResponseInputItem::FunctionCallOutput {
                call_id,
                output: FunctionCallOutputPayload {
                    content: format!("roots: {}, {}", roots[0], roots[1]),
                    success: Some(true),
                },
            }
        }
        Err(e) => ResponseInputItem::FunctionCallOutput {
            call_id,
            output: FunctionCallOutputPayload {
                content: format!("failed to parse function arguments: {e}"),
                success: Some(false),
            },
        },
    }
}

pub(crate) fn handle_matrix_det(arguments: String, call_id: String) -> ResponseInputItem {
    match serde_json::from_str::<MatrixArgs>(&arguments) {
        Ok(args) => {
            let m = args.matrix;
            let n = m.len();
            if !(n == 2 || n == 3) || !m.iter().all(|row| row.len() == n) {
                return ResponseInputItem::FunctionCallOutput {
                    call_id,
                    output: FunctionCallOutputPayload {
                        content: "error: matrix must be 2x2 or 3x3".to_string(),
                        success: Some(false),
                    },
                };
            }
            let det = if n == 2 {
                m[0][0] * m[1][1] - m[0][1] * m[1][0]
            } else {
                let a = m[0][0];
                let b = m[0][1];
                let c = m[0][2];
                let d = m[1][0];
                let e = m[1][1];
                let f = m[1][2];
                let g = m[2][0];
                let h = m[2][1];
                let i = m[2][2];
                a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)
            };
            ResponseInputItem::FunctionCallOutput {
                call_id,
                output: FunctionCallOutputPayload {
                    content: format!("determinant: {det}"),
                    success: Some(true),
                },
            }
        }
        Err(e) => ResponseInputItem::FunctionCallOutput {
            call_id,
            output: FunctionCallOutputPayload {
                content: format!("failed to parse function arguments: {e}"),
                success: Some(false),
            },
        },
    }
}
