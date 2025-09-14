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
        match serde_json::from_str::<Value>(text) {
            Ok(value) => {
                if let Err(errors) = TOOL_OUTPUT_SCHEMA.validate(&value) {
                    let msg = errors.map(|e| e.to_string()).collect::<Vec<_>>().join(", ");
                    return Err(error::CodexErr::Json(serde_json::Error::custom(msg)));
                }
                Ok(serde_json::from_value(value)?)
            }
            Err(_) => Ok(ToolMessage {
                kind: "message".into(),
                name: None,
                input: None,
                content: Some(text.to_string()),
            }),
        }
    }
}

impl ToolingBridge for OllamaToolBridge {
    fn wrap_prompt(&self, prompt: &mut Prompt) {
        let sys = ResponseItem::Message {
            id: None,
            role: "system".into(),
            content: vec![ContentItem::InputText {
                text: format!(
                    "Respond only with JSON following this schema:\n{TOOLING_SCHEMA}\nDo not include any prose outside of the JSON.",
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
                        let call = ResponseItem::CustomToolCall {
                            id: None,
                            status: None,
                            call_id: Uuid::new_v4().to_string(),
                            name,
                            input: input_str,
                        };
                        Ok(vec![ResponseEvent::OutputItemDone(call)])
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
    fn parse_event_parses_tool() {
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
            ResponseEvent::OutputItemDone(ResponseItem::CustomToolCall { name, input, .. }) => {
                assert_eq!(name, "test");
                assert_eq!(input, "{\"a\":1}");
            }
            _ => panic!("expected tool call"),
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
}
