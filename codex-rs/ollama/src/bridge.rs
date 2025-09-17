use codex_core::ContentItem;
use codex_core::Prompt;
use codex_core::ResponseEvent;
use codex_core::ResponseItem;
use codex_core::TOOLING_SCHEMA;
use codex_core::ToolingBridge;
use codex_core::error::Result;
use codex_core::error::{self};
use jsonschema::JSONSchema;
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde::de::Error as _;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use crate::json_repair::coerce_to_schema;
use crate::json_repair::extract_json_candidate;
use crate::json_repair::extract_patch_envelope;
use crate::json_repair::extract_single_shell_command;
use crate::json_repair::repair_json;

#[derive(Debug, Default)]
pub struct OllamaToolBridge;

#[expect(clippy::expect_used)]
static TOOL_OUTPUT_SCHEMA: Lazy<JSONSchema> = Lazy::new(|| {
    let schema_json: Value = serde_json::from_str(TOOLING_SCHEMA).expect("schema json");
    JSONSchema::compile(&schema_json).expect("compile schema")
});

#[derive(Deserialize)]
struct ToolMessage {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    input: Option<Value>,
    #[serde(default)]
    content: Option<String>,
}

impl OllamaToolBridge {
    fn parse_json(&self, text: &str) -> Result<ToolMessage> {
        // 1) Try a direct parse + schema validation.
        if let Ok(raw) = serde_json::from_str::<Value>(text) {
            // If the model echoed the schema document itself, ignore and continue.
            let looks_like_schema_doc = raw
                .as_object()
                .map(|o| {
                    o.contains_key("$schema")
                        || o.get("title").and_then(Value::as_str) == Some("Tooling output")
                })
                .unwrap_or(false);
            let has_type = raw
                .as_object()
                .and_then(|m| m.get("type"))
                .and_then(Value::as_str)
                .is_some();
            if !looks_like_schema_doc && TOOL_OUTPUT_SCHEMA.is_valid(&raw) {
                return Ok(serde_json::from_value(raw)?);
            }
            if !looks_like_schema_doc
                && let Some(coerced) = coerce_to_schema(raw)
                && TOOL_OUTPUT_SCHEMA.is_valid(&coerced)
            {
                return Ok(serde_json::from_value(coerced)?);
            }
            if !looks_like_schema_doc && has_type {
                // The model attempted structured output but it's invalid and couldn't be coerced.
                // Degrade gracefully to a plain assistant message instead of failing the stream.
                return Ok(ToolMessage {
                    kind: "message".into(),
                    name: None,
                    input: None,
                    content: Some(text.to_string()),
                });
            }
        }

        // 2) Extract the most plausible JSON candidate from fences/prose.
        let candidate = extract_json_candidate(text);
        if let Ok(raw) = serde_json::from_str::<Value>(&candidate) {
            let looks_like_schema_doc = raw
                .as_object()
                .map(|o| {
                    o.contains_key("$schema")
                        || o.get("title").and_then(Value::as_str) == Some("Tooling output")
                })
                .unwrap_or(false);
            let has_type = raw
                .as_object()
                .and_then(|m| m.get("type"))
                .and_then(Value::as_str)
                .is_some();
            if !looks_like_schema_doc && TOOL_OUTPUT_SCHEMA.is_valid(&raw) {
                return Ok(serde_json::from_value(raw)?);
            }
            if !looks_like_schema_doc
                && let Some(coerced) = coerce_to_schema(raw)
                && TOOL_OUTPUT_SCHEMA.is_valid(&coerced)
            {
                return Ok(serde_json::from_value(coerced)?);
            }
            if !looks_like_schema_doc && has_type {
                return Ok(ToolMessage {
                    kind: "message".into(),
                    name: None,
                    input: None,
                    content: Some(text.to_string()),
                });
            }
        }

        // 3) Attempt automated JSON repair on the candidate, then re-parse.
        let repaired = repair_json(&candidate);
        if let Ok(raw) = serde_json::from_str::<Value>(&repaired) {
            let looks_like_schema_doc = raw
                .as_object()
                .map(|o| {
                    o.contains_key("$schema")
                        || o.get("title").and_then(Value::as_str) == Some("Tooling output")
                })
                .unwrap_or(false);
            let has_type = raw
                .as_object()
                .and_then(|m| m.get("type"))
                .and_then(Value::as_str)
                .is_some();
            if !looks_like_schema_doc && TOOL_OUTPUT_SCHEMA.is_valid(&raw) {
                return Ok(serde_json::from_value(raw)?);
            }
            if !looks_like_schema_doc
                && let Some(coerced) = coerce_to_schema(raw)
                && TOOL_OUTPUT_SCHEMA.is_valid(&coerced)
            {
                return Ok(serde_json::from_value(coerced)?);
            }
            if !looks_like_schema_doc && has_type {
                return Ok(ToolMessage {
                    kind: "message".into(),
                    name: None,
                    input: None,
                    content: Some(text.to_string()),
                });
            }
        }

        // 4) Heuristic: detect apply_patch envelopes embedded in prose and
        // convert into a tool call.
        if let Some(patch) = extract_patch_envelope(text) {
            return Ok(ToolMessage {
                kind: "tool".into(),
                name: Some("apply_patch".into()),
                input: Some(serde_json::json!({"input": patch})),
                content: None,
            });
        }

        // 5) Heuristic: a single fenced or inline command like
        //    ```bash\npython t.py --help\n``` → shell tool call.
        if let Some(argv) = extract_single_shell_command(text) {
            return Ok(ToolMessage {
                kind: "tool".into(),
                name: Some("shell".into()),
                input: Some(serde_json::json!({
                    "command": argv,
                })),
                content: None,
            });
        }

        // 6) Give up and return the original text as an assistant message.
        Ok(ToolMessage {
            kind: "message".into(),
            name: None,
            input: None,
            content: Some(text.to_string()),
        })
    }
}

// Legacy helpers were in this file; logic moved into json_repair for reuse.

impl ToolingBridge for OllamaToolBridge {
    fn wrap_prompt(&self, prompt: &mut Prompt) {
        let sys = ResponseItem::Message {
            id: None,
            role: "system".into(),
            content: vec![ContentItem::InputText {
                text: format!(
                    "Respond only with JSON following this schema:\n{TOOLING_SCHEMA}\n\
Do not include any prose, code fences, or XML/HTML wrappers.\n\
When you need to act, always return: {{\"type\":\"tool\",\"name\":\"<tool>\",\"input\":{{...}}}}.\n\
When you need to reply, return: {{\"type\":\"message\",\"content\":\"<text>\"}}.\n\
Available tools:\n- shell: input = {{ \"command\": [\"<exe>\", \"arg1\", ...], \"workdir\"?: string, \"timeout_ms\"?: number }}\n- apply_patch: input = {{ \"input\": string }} where string is an exact patch envelope:\n*** Begin Patch\n*** Add File: path/to/file\n+<file contents>\n*** End Patch\n\
Do not invent tool names. Use only shell and apply_patch."
                ),
            }],
        };
        prompt.input.insert(0, sys);
    }

    fn parse_event(&self, event: ResponseEvent) -> Result<Vec<ResponseEvent>> {
        match event {
            ResponseEvent::OutputItemDone(ResponseItem::Message { role, content, .. })
                if role == "assistant" =>
            {
                let text = content
                    .iter()
                    .find_map(|c| match c {
                        ContentItem::OutputText { text } => Some(text.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();
                let parsed = self.parse_json(&text)?;
                match parsed.kind.as_str() {
                    "message" => {
                        let msg = ResponseItem::Message {
                            id: None,
                            role: "assistant".into(),
                            content: vec![ContentItem::OutputText {
                                text: parsed.content.unwrap_or_default(),
                            }],
                        };
                        Ok(vec![ResponseEvent::OutputItemDone(msg)])
                    }
                    "tool" => {
                        let name = parsed.name.unwrap_or_default();
                        let input_val = parsed.input.unwrap_or(Value::Null);
                        let input_str = serde_json::to_string(&input_val)?;

                        // Route built‑in tools (apply_patch, shell) via FunctionCall.
                        // Anything else is treated as a CustomToolCall so it can be
                        // dispatched through MCP or other custom handlers.
                        let call_id = Uuid::new_v4().to_string();
                        if name == "apply_patch" || name == "shell" {
                            let call = ResponseItem::FunctionCall {
                                id: None,
                                name,
                                arguments: input_str,
                                call_id,
                            };
                            Ok(vec![ResponseEvent::OutputItemDone(call)])
                        } else {
                            let call = ResponseItem::CustomToolCall {
                                id: None,
                                status: None,
                                call_id,
                                name,
                                input: input_str,
                            };
                            Ok(vec![ResponseEvent::OutputItemDone(call)])
                        }
                    }
                    _ => Err(error::CodexErr::Json(serde_json::Error::custom(format!(
                        "unknown type: {}",
                        parsed.kind
                    )))),
                }
            }
            other => Ok(vec![other]),
        }
    }
}

pub fn register_ollama_tool_bridge() {
    codex_core::register_tooling_bridge("ollama", || Arc::new(OllamaToolBridge));
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_core::ContentItem;
    use codex_core::Prompt;
    use codex_core::ResponseItem;
    use pretty_assertions::assert_eq;

    #[test]
    fn wrap_prompt_prepends_system_message() {
        let mut prompt = Prompt::default();
        OllamaToolBridge.wrap_prompt(&mut prompt);
        match &prompt.input[0] {
            ResponseItem::Message { role, content, .. } => {
                assert_eq!(role, "system");
                if let ContentItem::InputText { text } = &content[0] {
                    assert!(text.contains("Respond only with JSON"));
                } else {
                    panic!("expected InputText");
                }
            }
            _ => panic!("expected system message"),
        }
    }

    #[test]
    fn parse_event_parses_message() {
        let bridge = OllamaToolBridge;
        let item = ResponseItem::Message {
            id: None,
            role: "assistant".into(),
            content: vec![ContentItem::OutputText {
                text: "{\"type\":\"message\",\"content\":\"hi\"}".into(),
            }],
        };
        let events = bridge
            .parse_event(ResponseEvent::OutputItemDone(item))
            .expect("decode");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ResponseEvent::OutputItemDone(ResponseItem::Message { content, .. }) => {
                if let ContentItem::OutputText { text } = &content[0] {
                    assert_eq!(text, "hi");
                } else {
                    panic!("expected OutputText");
                }
            }
            _ => panic!("expected message"),
        }
    }

    #[test]
    fn parse_event_parses_tool_to_function_call() {
        let bridge = OllamaToolBridge;
        let item = ResponseItem::Message {
            id: None,
            role: "assistant".into(),
            content: vec![ContentItem::OutputText {
                text: "{\"type\":\"tool\",\"name\":\"test\",\"input\":{\"a\":1}}".into(),
            }],
        };
        let events = bridge
            .parse_event(ResponseEvent::OutputItemDone(item))
            .expect("decode");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ResponseEvent::OutputItemDone(ResponseItem::FunctionCall {
                name, arguments, ..
            }) => {
                assert_eq!(name, "test");
                assert_eq!(arguments, "{\"a\":1}");
            }
            _ => panic!("expected function call"),
        }
    }

    #[test]
    fn parse_event_wraps_plain_text() {
        let bridge = OllamaToolBridge;
        let item = ResponseItem::Message {
            id: None,
            role: "assistant".into(),
            content: vec![ContentItem::OutputText {
                text: "plain".into(),
            }],
        };
        let events = bridge
            .parse_event(ResponseEvent::OutputItemDone(item))
            .expect("decode");
        match &events[0] {
            ResponseEvent::OutputItemDone(ResponseItem::Message { content, .. }) => {
                if let ContentItem::OutputText { text } = &content[0] {
                    assert_eq!(text, "plain");
                } else {
                    panic!("expected OutputText");
                }
            }
            _ => panic!("expected message"),
        }
    }

    #[test]
    fn extract_json_from_code_fence() {
        let bridge = OllamaToolBridge;
        let item = ResponseItem::Message {
            id: None,
            role: "assistant".into(),
            content: vec![ContentItem::OutputText {
                text: "```json\n{\n  \"type\": \"tool\",\n  \"name\": \"shell\",\n  \"input\": { \"command\": [\"echo\", \"hi\"] }\n}\n```".into(),
            }],
        };
        let events = bridge
            .parse_event(ResponseEvent::OutputItemDone(item))
            .expect("decode");
        match &events[0] {
            ResponseEvent::OutputItemDone(ResponseItem::FunctionCall {
                name, arguments, ..
            }) => {
                assert_eq!(name, "shell");
                assert!(arguments.contains("echo"));
                assert!(arguments.contains("hi"));
            }
            _ => panic!("expected function call"),
        }
    }

    #[test]
    fn repairs_single_quotes_and_unquoted_keys() {
        let bridge = OllamaToolBridge;
        let raw = "{'type': 'tool', name: 'shell', input: {'command': ['echo', 'hi']},}";
        let item = ResponseItem::Message {
            id: None,
            role: "assistant".into(),
            content: vec![ContentItem::OutputText { text: raw.into() }],
        };
        let events = bridge
            .parse_event(ResponseEvent::OutputItemDone(item))
            .expect("decode");
        match &events[0] {
            ResponseEvent::OutputItemDone(ResponseItem::FunctionCall {
                name, arguments, ..
            }) => {
                assert_eq!(name, "shell");
                assert!(arguments.contains("echo"));
                assert!(arguments.contains("hi"));
            }
            _ => panic!("expected function call"),
        }
    }

    #[test]
    fn repairs_missing_type_with_arguments_alias() {
        let bridge = OllamaToolBridge;
        // Missing `type`, uses `arguments` instead of `input`.
        let raw = "{\"name\": \"shell\", \"arguments\": {\"command\": [\"echo\", \"ok\"]}}";
        let item = ResponseItem::Message {
            id: None,
            role: "assistant".into(),
            content: vec![ContentItem::OutputText { text: raw.into() }],
        };
        let events = bridge
            .parse_event(ResponseEvent::OutputItemDone(item))
            .expect("decode");
        match &events[0] {
            ResponseEvent::OutputItemDone(ResponseItem::FunctionCall {
                name, arguments, ..
            }) => {
                assert_eq!(name, "shell");
                assert!(arguments.contains("ok"));
            }
            _ => panic!("expected function call"),
        }
    }

    #[test]
    fn converts_single_fenced_command_to_shell_tool() {
        let bridge = OllamaToolBridge;
        let item = ResponseItem::Message {
            id: None,
            role: "assistant".into(),
            content: vec![ContentItem::OutputText {
                text: "```bash\npython tictactoe.py --help\n```".into(),
            }],
        };
        let events = bridge
            .parse_event(ResponseEvent::OutputItemDone(item))
            .expect("decode");
        match &events[0] {
            ResponseEvent::OutputItemDone(ResponseItem::FunctionCall {
                name, arguments, ..
            }) => {
                assert_eq!(name, "shell");
                assert!(arguments.contains("tictactoe.py"));
                assert!(arguments.contains("--help"));
            }
            _ => panic!("expected shell function call"),
        }
    }
}
