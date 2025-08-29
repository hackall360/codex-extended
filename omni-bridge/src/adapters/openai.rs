use crate::{
    Adapter, Caps, ContentPart, FinishReason, OmniDelta, OmniError, OmniMessage, OmniRequest,
    OmniResponse, OmniTool, ResponseFormat, Role, Sampling, ToolChoice, Usage,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct OpenAIAdapter;

impl OpenAIAdapter {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

#[async_trait]
impl Adapter for OpenAIAdapter {
    fn id(&self) -> &'static str {
        "openai"
    }
    fn capabilities(&self) -> Caps {
        Caps::CHAT
            | Caps::TOOLS
            | Caps::JSON_MODE
            | Caps::JSON_SCHEMA
            | Caps::SYSTEM_MSG
            | Caps::STREAMING
            | Caps::VISION
    }

    fn decode_request(&self, native: &Value) -> Result<OmniRequest, OmniError> {
        let model = native
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let mut messages = Vec::new();
        if let Some(arr) = native.get("messages").and_then(Value::as_array) {
            for m in arr {
                let role = match m.get("role").and_then(Value::as_str).unwrap_or("user") {
                    "system" => Role::System,
                    "user" => Role::User,
                    "assistant" => Role::Assistant,
                    _ => Role::User,
                };
                let content = match m.get("content") {
                    Some(Value::String(t)) => vec![ContentPart::Text { text: t.clone() }],
                    Some(Value::Array(parts)) => {
                        let mut v = Vec::new();
                        for p in parts {
                            match p.get("type").and_then(Value::as_str) {
                                Some("text") => v.push(ContentPart::Text {
                                    text: p
                                        .get("text")
                                        .and_then(Value::as_str)
                                        .unwrap_or("")
                                        .to_string(),
                                }),
                                Some("image_url") => {
                                    let url = p
                                        .get("image_url")
                                        .and_then(|iu| iu.get("url"))
                                        .and_then(Value::as_str)
                                        .unwrap_or("")
                                        .to_string();
                                    v.push(ContentPart::ImageUrl { url, mime: None });
                                }
                                _ => {}
                            }
                        }
                        v
                    }
                    _ => vec![],
                };

                let mut content = content;
                if let Some(tt) = m.get("tool_calls").and_then(Value::as_array) {
                    for (i, tc) in tt.iter().enumerate() {
                        let id = tc
                            .get("id")
                            .and_then(Value::as_str)
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("tc_{i}"));
                        let func = tc.get("function").cloned().unwrap_or(json!({}));
                        let name = func
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let arguments = func
                            .get("arguments")
                            .map(|v| {
                                if v.is_string() {
                                    serde_json::from_str::<Value>(v.as_str().unwrap_or("{}"))
                                        .unwrap_or(json!({}))
                                } else {
                                    v.clone()
                                }
                            })
                            .unwrap_or(json!({}));
                        content.push(ContentPart::ToolUse {
                            id,
                            name,
                            arguments,
                        });
                    }
                }
                if role == Role::Tool {
                    let id = m
                        .get("tool_call_id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let c = m
                        .get("content")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    content = vec![ContentPart::ToolResult {
                        tool_use_id: id,
                        content: vec![ContentPart::Text { text: c }],
                    }];
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
                        if t.get("type").and_then(Value::as_str) == Some("function") {
                            let f = t.get("function")?;
                            Some(OmniTool {
                                name: f.get("name")?.as_str()?.to_string(),
                                description: f
                                    .get("description")
                                    .and_then(Value::as_str)
                                    .map(|s| s.to_string()),
                                parameters: f.get("parameters").cloned().unwrap_or(json!({})),
                            })
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let tool_choice = match native.get("tool_choice") {
            Some(Value::String(s)) if s == "none" => Some(ToolChoice::None),
            Some(Value::String(s)) if s == "auto" => Some(ToolChoice::Auto),
            Some(Value::Object(o)) if o.get("type").and_then(Value::as_str) == Some("function") => {
                o.get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(Value::as_str)
                    .map(|n| ToolChoice::Named {
                        name: n.to_string(),
                    })
            }
            _ => None,
        };

        let response_format = match native.get("response_format") {
            Some(Value::String(s)) if s == "json_object" => Some(crate::ResponseFormat::JsonObject),
            Some(Value::Object(o))
                if o.get("type").and_then(Value::as_str) == Some("json_schema") =>
            {
                let name = o
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("schema")
                    .to_string();
                let schema = o.get("json_schema").cloned().unwrap_or(json!({}));
                Some(crate::ResponseFormat::JsonSchema { name, schema })
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
            top_k: None,
            max_tokens: native
                .get("max_tokens")
                .and_then(Value::as_u64)
                .map(|v| v as u32),
            presence_penalty: native
                .get("presence_penalty")
                .and_then(Value::as_f64)
                .map(|v| v as f32),
            frequency_penalty: native
                .get("frequency_penalty")
                .and_then(Value::as_f64)
                .map(|v| v as f32),
            stop: native
                .get("stop")
                .map(|s| {
                    if s.is_string() {
                        vec![s.as_str().unwrap().to_string()]
                    } else {
                        s.as_array()
                            .unwrap_or(&vec![])
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    }
                })
                .unwrap_or_default(),
            seed: native.get("seed").and_then(Value::as_u64),
        };

        Ok(OmniRequest {
            model,
            messages,
            tools,
            tool_choice,
            response_format,
            sampling,
            metadata: json!({"native":"openai"}),
        })
    }

    fn encode_request(&self, ir: &OmniRequest) -> Result<Value, OmniError> {
        let messages: Vec<Value> = ir.messages.iter().map(|m| {
            let role = match m.role { Role::System=>"system", Role::User=>"user", Role::Assistant=>"assistant", Role::Tool=>"tool" };
            let (mut content_arr, mut tool_calls, mut tool_role_extras) = (Vec::new(), Vec::new(), None::<Value>);
            for part in &m.content {
                match part {
                    ContentPart::Text { text } => content_arr.push(json!({"type":"text","text":text})),
                    ContentPart::ImageUrl { url, .. } => content_arr.push(json!({"type":"image_url","image_url":{"url":url}})),
                    ContentPart::ToolUse { id, name, arguments } => {
                        tool_calls.push(json!({"id": id, "type":"function", "function":{"name": name, "arguments": serde_json::to_string(arguments).unwrap_or("{}".into())}}));
                    }
                    ContentPart::ToolResult { tool_use_id, content } => {
                        let text = content.iter().filter_map(|c| if let ContentPart::Text { text } = c { Some(text.as_str()) } else { None }).collect::<Vec<_>>().join("");
                        tool_role_extras = Some(json!({"tool_call_id": tool_use_id, "content": text}));
                    }
                }
            }
            let mut v = json!({"role": role});
            if !content_arr.is_empty() && role != "tool" {
                v["content"] = Value::Array(content_arr);
            }
            if !tool_calls.is_empty() {
                v["tool_calls"] = Value::Array(tool_calls);
            }
            if role == "tool" {
                if let Some(extra) = tool_role_extras { v.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone()); }
            }
            v
        }).collect();

        let tools: Vec<Value> = ir.tools.iter().map(|t| {
            json!({"type":"function","function":{"name":t.name,"description":t.description,"parameters":t.parameters}})
        }).collect();

        let tool_choice = match &ir.tool_choice {
            Some(ToolChoice::Auto) => json!("auto"),
            Some(ToolChoice::None) => json!("none"),
            Some(ToolChoice::Required) => json!({"type":"function","function":{"name":null}}),
            Some(ToolChoice::Named { name }) => json!({"type":"function","function":{"name":name}}),
            None => Value::Null,
        };

        let mut req = json!({
            "model": ir.model,
            "messages": messages,
            "temperature": ir.sampling.temperature,
            "top_p": ir.sampling.top_p,
            "max_tokens": ir.sampling.max_tokens,
            "presence_penalty": ir.sampling.presence_penalty,
            "frequency_penalty": ir.sampling.frequency_penalty,
        });
        if !tools.is_empty() {
            req["tools"] = Value::Array(tools);
        }
        if !ir.sampling.stop.is_empty() {
            req["stop"] = Value::Array(
                ir.sampling
                    .stop
                    .iter()
                    .map(|s| Value::String(s.clone()))
                    .collect(),
            );
        }
        if let Some(tc) = ir.tool_choice.as_ref() {
            if !matches!(tc, ToolChoice::Auto) {
                req["tool_choice"] = tool_choice;
            }
        }
        if let Some(ResponseFormat::JsonObject) = &ir.response_format {
            req["response_format"] = json!({"type":"json_object"});
        } else if let Some(ResponseFormat::JsonSchema { name, schema }) = &ir.response_format {
            req["response_format"] = json!({"type":"json_schema","name":name,"json_schema":schema});
        }
        Ok(req)
    }

    fn decode_response(&self, native: &Value) -> Result<OmniResponse, OmniError> {
        let choice = native
            .pointer("/choices/0/message")
            .ok_or_else(|| OmniError::BadRequest("missing choices[0]".into()))?;
        let role = match choice
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("assistant")
        {
            "assistant" => Role::Assistant,
            "tool" => Role::Tool,
            "user" => Role::User,
            "system" => Role::System,
            _ => Role::Assistant,
        };
        let mut content: Vec<ContentPart> = Vec::new();
        match choice.get("content") {
            Some(Value::String(s)) => content.push(ContentPart::Text { text: s.clone() }),
            Some(Value::Array(parts)) => {
                for p in parts {
                    match p.get("type").and_then(Value::as_str) {
                        Some("text") => content.push(ContentPart::Text {
                            text: p
                                .get("text")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                        }),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        if let Some(tcs) = choice.get("tool_calls").and_then(Value::as_array) {
            for tc in tcs {
                let id = tc
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let f = tc.get("function").cloned().unwrap_or(json!({}));
                let name = f
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let arguments = f
                    .get("arguments")
                    .map(|v| {
                        if v.is_string() {
                            serde_json::from_str(v.as_str().unwrap_or("{}")).unwrap_or(json!({}))
                        } else {
                            v.clone()
                        }
                    })
                    .unwrap_or(json!({}));
                content.push(ContentPart::ToolUse {
                    id,
                    name,
                    arguments,
                });
            }
        }
        let prompt_t = native
            .pointer("/usage/prompt_tokens")
            .and_then(Value::as_u64)
            .map(|v| v as u32);
        let comp_t = native
            .pointer("/usage/completion_tokens")
            .and_then(Value::as_u64)
            .map(|v| v as u32);
        let total_t = native
            .pointer("/usage/total_tokens")
            .and_then(Value::as_u64)
            .map(|v| v as u32);
        let finish_reason = match native
            .pointer("/choices/0/finish_reason")
            .and_then(Value::as_str)
            .unwrap_or("stop")
        {
            "stop" => FinishReason::Stop,
            "length" => FinishReason::Length,
            "tool_calls" => FinishReason::ToolCalls,
            x => FinishReason::Other(x.to_string()),
        };
        Ok(OmniResponse {
            message: OmniMessage { role, content },
            usage: Usage {
                prompt_tokens: prompt_t,
                completion_tokens: comp_t,
                total_tokens: total_t,
            },
            finish_reason,
        })
    }

    fn stream_decode(&self, chunk: &[u8]) -> Result<Vec<OmniDelta>, OmniError> {
        let s = std::str::from_utf8(chunk).map_err(|e| OmniError::Io(e.to_string()))?;
        let mut out = Vec::new();
        for line in s.lines().filter(|l| l.starts_with("data: ")) {
            let data = &line[6..];
            if data == "[DONE]" {
                out.push(OmniDelta::Finish {
                    reason: FinishReason::Stop,
                    usage: None,
                });
                continue;
            }
            let v: Value = serde_json::from_str(data)?;
            if let Some(delta) = v.pointer("/choices/0/delta") {
                if let Some(t) = delta.get("content").and_then(Value::as_str) {
                    if !t.is_empty() {
                        out.push(OmniDelta::Text {
                            text: t.to_string(),
                        });
                    }
                }
                if let Some(tc) = delta
                    .get("tool_calls")
                    .and_then(Value::as_array)
                    .and_then(|a| a.get(0))
                {
                    let id = tc
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let f = tc.get("function").cloned().unwrap_or(json!({}));
                    if let Some(name) = f.get("name").and_then(Value::as_str) {
                        out.push(OmniDelta::ToolUseStart {
                            id: id.clone(),
                            name: name.to_string(),
                        });
                    }
                    if let Some(args) = f.get("arguments").and_then(Value::as_str) {
                        out.push(OmniDelta::ToolUseArgs {
                            id: id.clone(),
                            chunk: args.to_string(),
                        });
                    }
                }
            }
            if let Some(reason) = v
                .pointer("/choices/0/finish_reason")
                .and_then(Value::as_str)
            {
                let fr = match reason {
                    "stop" => FinishReason::Stop,
                    "length" => FinishReason::Length,
                    "tool_calls" => FinishReason::ToolCalls,
                    x => FinishReason::Other(x.to_string()),
                };
                out.push(OmniDelta::Finish {
                    reason: fr,
                    usage: None,
                });
            }
        }
        Ok(out)
    }

    fn stream_encode(&self, deltas: &[OmniDelta]) -> Result<Vec<Vec<u8>>, OmniError> {
        let mut out = Vec::new();
        for d in deltas {
            let payload = match d {
                OmniDelta::Text { text } => {
                    json!({"choices":[{"delta":{"content":text},"index":0,"finish_reason":null}]})
                }
                OmniDelta::ToolUseStart { id, name } => {
                    json!({"choices":[{"delta":{"tool_calls":[{"id":id,"type":"function","function":{"name":name}}]},"index":0}]})
                }
                OmniDelta::ToolUseArgs { id, chunk } => {
                    json!({"choices":[{"delta":{"tool_calls":[{"id":id,"type":"function","function":{"arguments":chunk}}]},"index":0}]})
                }
                OmniDelta::ToolUseEnd { .. } => continue,
                OmniDelta::Finish { reason, .. } => {
                    let r = match reason {
                        FinishReason::Stop => "stop",
                        FinishReason::Length => "length",
                        FinishReason::ToolCalls => "tool_calls",
                        FinishReason::ContentFilter => "content_filter",
                        FinishReason::Other(x) => x.as_str(),
                    };
                    json!({"choices":[{"delta":{},"index":0,"finish_reason":r}]})
                }
            };
            out.push(format!("data: {}\n\n", payload).into_bytes());
        }
        out.push(b"data: [DONE]\n\n".to_vec());
        Ok(out)
    }
}
