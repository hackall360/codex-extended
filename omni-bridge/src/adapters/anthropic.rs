use crate::*;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct AnthropicAdapter;
impl AnthropicAdapter {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

#[async_trait]
impl Adapter for AnthropicAdapter {
    fn id(&self) -> &'static str {
        "anthropic"
    }
    fn capabilities(&self) -> Caps {
        Caps::CHAT
            | Caps::TOOLS
            | Caps::SYSTEM_MSG
            | Caps::STREAMING
            | Caps::VISION
            | Caps::JSON_SCHEMA
    }

    fn decode_request(&self, native: &Value) -> Result<OmniRequest, OmniError> {
        let model = native
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let mut messages: Vec<OmniMessage> = Vec::new();

        if let Some(sys) = native.get("system").and_then(Value::as_str) {
            messages.push(OmniMessage {
                role: Role::System,
                content: vec![ContentPart::Text {
                    text: sys.to_string(),
                }],
            });
        }

        if let Some(arr) = native.get("messages").and_then(Value::as_array) {
            for m in arr {
                let role = match m.get("role").and_then(Value::as_str).unwrap_or("user") {
                    "user" => Role::User,
                    "assistant" => Role::Assistant,
                    _ => Role::User,
                };
                let mut content = Vec::new();
                if let Some(blocks) = m.get("content").and_then(Value::as_array) {
                    for b in blocks {
                        match b.get("type").and_then(Value::as_str) {
                            Some("text") => content.push(ContentPart::Text {
                                text: b
                                    .get("text")
                                    .and_then(Value::as_str)
                                    .unwrap_or("")
                                    .to_string(),
                            }),
                            Some("tool_use") => {
                                content.push(ContentPart::ToolUse {
                                    id: b
                                        .get("id")
                                        .and_then(Value::as_str)
                                        .unwrap_or("")
                                        .to_string(),
                                    name: b
                                        .get("name")
                                        .and_then(Value::as_str)
                                        .unwrap_or("")
                                        .to_string(),
                                    arguments: b.get("input").cloned().unwrap_or(json!({})),
                                });
                            }
                            Some("tool_result") => {
                                let id = b
                                    .get("tool_use_id")
                                    .and_then(Value::as_str)
                                    .unwrap_or("")
                                    .to_string();
                                let txt = b
                                    .get("content")
                                    .and_then(Value::as_str)
                                    .unwrap_or("")
                                    .to_string();
                                content.push(ContentPart::ToolResult {
                                    tool_use_id: id,
                                    content: vec![ContentPart::Text { text: txt }],
                                });
                            }
                            _ => {}
                        }
                    }
                }
                messages.push(OmniMessage { role, content });
            }
        }

        let tools = native
            .get("tools")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| {
                        Some(OmniTool {
                            name: t.get("name")?.as_str()?.to_string(),
                            description: t
                                .get("description")
                                .and_then(Value::as_str)
                                .map(|s| s.to_string()),
                            parameters: t.get("input_schema").cloned().unwrap_or(json!({})),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let tool_choice = match native.get("tool_choice") {
            Some(Value::String(s)) if s == "auto" => Some(ToolChoice::Auto),
            None => Some(ToolChoice::Auto),
            Some(Value::String(s)) if s == "none" => Some(ToolChoice::None),
            Some(Value::Object(o)) => {
                o.get("name")
                    .and_then(Value::as_str)
                    .map(|n| ToolChoice::Named {
                        name: n.to_string(),
                    })
            }
            _ => None,
        };

        let sampling = Sampling {
            temperature: native
                .get("temperature")
                .and_then(Value::as_f64)
                .map(|v| v as f32),
            top_p: native
                .get("top_p")
                .and_then(Value::as_f64)
                .map(|v| v as f32),
            top_k: native
                .get("top_k")
                .and_then(Value::as_u64)
                .map(|v| v as u32),
            max_tokens: native
                .get("max_tokens")
                .and_then(Value::as_u64)
                .map(|v| v as u32),
            presence_penalty: None,
            frequency_penalty: None,
            stop: vec![],
            seed: native.get("seed").and_then(Value::as_u64),
        };

        Ok(OmniRequest {
            model,
            messages,
            tools,
            tool_choice,
            response_format: None,
            sampling,
            metadata: json!({"native":"anthropic"}),
        })
    }

    fn encode_request(&self, ir: &OmniRequest) -> Result<Value, OmniError> {
        let mut system: Option<String> = None;
        let mut msgs: Vec<Value> = Vec::new();
        for m in &ir.messages {
            match m.role {
                Role::System => {
                    let s = m
                        .content
                        .iter()
                        .filter_map(|c| {
                            if let ContentPart::Text { text } = c {
                                Some(text.as_str())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    system = Some(s);
                }
                Role::User | Role::Assistant | Role::Tool => {
                    let role = match m.role {
                        Role::User => "user",
                        Role::Assistant => "assistant",
                        Role::Tool => "user",
                        _ => unreachable!(),
                    };
                    let mut content: Vec<Value> = Vec::new();
                    for c in &m.content {
                        match c {
                            ContentPart::Text { text } => {
                                content.push(json!({"type":"text","text":text}))
                            }
                            ContentPart::ImageUrl { url, .. } => content
                                .push(json!({"type":"image","source":{"type":"url","url":url}})),
                            ContentPart::ToolUse {
                                id,
                                name,
                                arguments,
                            } => content.push(
                                json!({"type":"tool_use","id":id,"name":name,"input":arguments}),
                            ),
                            ContentPart::ToolResult {
                                tool_use_id,
                                content: parts,
                            } => {
                                let txt = parts
                                    .iter()
                                    .filter_map(|p| {
                                        if let ContentPart::Text { text } = p {
                                            Some(text.as_str())
                                        } else {
                                            None
                                        }
                                    })
                                    .collect::<Vec<_>>()
                                    .join("");
                                content.push(json!({"type":"tool_result","tool_use_id":tool_use_id,"content":txt}));
                            }
                        }
                    }
                    msgs.push(json!({"role": role, "content": content}));
                }
            }
        }

        let tools: Vec<Value> = ir
            .tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.name, "description": t.description, "input_schema": t.parameters
                })
            })
            .collect();

        let tool_choice = match &ir.tool_choice {
            Some(ToolChoice::None) => json!("none"),
            Some(ToolChoice::Auto) | None => json!("auto"),
            Some(ToolChoice::Required) => json!({"type":"any"}),
            Some(ToolChoice::Named { name }) => json!({"type":"tool","name":name}),
        };

        let mut req = json!({
            "model": ir.model,
            "messages": msgs,
            "max_tokens": ir.sampling.max_tokens.unwrap_or(1024),
            "temperature": ir.sampling.temperature,
            "top_p": ir.sampling.top_p,
            "top_k": ir.sampling.top_k,
            "tools": tools,
            "tool_choice": tool_choice
        });
        if let Some(s) = system {
            req["system"] = Value::String(s);
        }
        Ok(req)
    }

    fn decode_response(&self, native: &Value) -> Result<OmniResponse, OmniError> {
        let content = native
            .get("content")
            .and_then(Value::as_array)
            .ok_or_else(|| OmniError::BadRequest("no content".into()))?;
        let mut parts: Vec<ContentPart> = Vec::new();
        for b in content {
            match b.get("type").and_then(Value::as_str) {
                Some("text") => parts.push(ContentPart::Text {
                    text: b
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                }),
                Some("tool_use") => parts.push(ContentPart::ToolUse {
                    id: b
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    name: b
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    arguments: b.get("input").cloned().unwrap_or(json!({})),
                }),
                _ => {}
            }
        }
        let usage = Usage {
            prompt_tokens: native
                .pointer("/usage/input_tokens")
                .and_then(Value::as_u64)
                .map(|v| v as u32),
            completion_tokens: native
                .pointer("/usage/output_tokens")
                .and_then(Value::as_u64)
                .map(|v| v as u32),
            total_tokens: native
                .pointer("/usage/total_tokens")
                .and_then(Value::as_u64)
                .map(|v| v as u32),
        };
        let stop_reason = native
            .get("stop_reason")
            .and_then(Value::as_str)
            .unwrap_or("end_turn");
        let fr = match stop_reason {
            "end_turn" => FinishReason::Stop,
            "max_tokens" => FinishReason::Length,
            "tool_use" => FinishReason::ToolCalls,
            x => FinishReason::Other(x.to_string()),
        };
        Ok(OmniResponse {
            message: OmniMessage {
                role: Role::Assistant,
                content: parts,
            },
            usage,
            finish_reason: fr,
        })
    }

    fn stream_decode(&self, chunk: &[u8]) -> Result<Vec<OmniDelta>, OmniError> {
        let s = std::str::from_utf8(chunk).map_err(|e| OmniError::Io(e.to_string()))?;
        let mut out = Vec::new();
        for line in s.lines().filter(|l| !l.is_empty()) {
            let v: Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            match v
                .get("type")
                .and_then(Value::as_str)
                .or_else(|| v.get("event").and_then(Value::as_str))
            {
                Some("content_block_delta") => {
                    if v.pointer("/delta/type").and_then(Value::as_str) == Some("text_delta") {
                        if let Some(t) = v.pointer("/delta/text").and_then(Value::as_str) {
                            out.push(OmniDelta::Text {
                                text: t.to_string(),
                            });
                        }
                    }
                }
                Some("content_block_start") => {
                    if v.pointer("/content_block/type").and_then(Value::as_str) == Some("tool_use")
                    {
                        let id = v
                            .pointer("/content_block/id")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let name = v
                            .pointer("/content_block/name")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        out.push(OmniDelta::ToolUseStart { id, name });
                    }
                }
                Some("message_stop") => out.push(OmniDelta::Finish {
                    reason: FinishReason::Stop,
                    usage: None,
                }),
                _ => {}
            }
        }
        Ok(out)
    }

    fn stream_encode(&self, deltas: &[OmniDelta]) -> Result<Vec<Vec<u8>>, OmniError> {
        let mut out = Vec::new();
        for d in deltas {
            match d {
                OmniDelta::Text { text } => out.push(format!(r#"{{"type":"content_block_delta","delta":{{"type":"text_delta","text":"{}"}}}}"#, text.replace('"', "\\\"" )).into_bytes()),
                OmniDelta::ToolUseStart { id, name } =>
                    out.push(format!(r#"{{"type":"content_block_start","content_block":{{"type":"tool_use","id":"{}","name":"{}"}}}}"#, id, name).into_bytes()),
                OmniDelta::ToolUseArgs { .. } => {},
                OmniDelta::ToolUseEnd { .. } => {},
                OmniDelta::Finish { .. } => out.push(br#"{"type":"message_stop"}"#.to_vec()),
            }
            out.push(b"\n".to_vec());
        }
        Ok(out)
    }
}
