use crate::*;
use serde_json::json;

#[cfg(feature = "anthropic")]
pub mod anthropic;
#[cfg(feature = "gemini")]
pub mod gemini;
#[cfg(feature = "openai")]
pub mod openai;

pub fn encode_response_native(
    target: &dyn Adapter,
    ir: &OmniResponse,
) -> Result<serde_json::Value, OmniError> {
    match target.id() {
        "openai" => Ok(json!({
            "id": "chatcmpl_fake",
            "object": "chat.completion",
            "choices": [{
              "index": 0,
              "finish_reason": match ir.finish_reason {
                  FinishReason::Stop => "stop",
                  FinishReason::Length => "length",
                  FinishReason::ToolCalls => "tool_calls",
                  FinishReason::ContentFilter => "content_filter",
                  FinishReason::Other(_) => "stop"
              },
              "message": {
                "role": "assistant",
                "content": ir.message.content.iter().filter_map(|p| match p {
                    ContentPart::Text { text } => Some(json!({"type":"text","text":text})),
                    _ => None
                }).collect::<Vec<_>>(),
                "tool_calls": ir.message.content.iter().filter_map(|p| match p {
                    ContentPart::ToolUse { id, name, arguments } => Some(json!({"id":id,"type":"function","function":{"name":name,"arguments":serde_json::to_string(arguments).unwrap_or("{}".into())}})),
                    _ => None
                }).collect::<Vec<_>>(),
              }
            }],
            "usage": {
                "prompt_tokens": ir.usage.prompt_tokens,
                "completion_tokens": ir.usage.completion_tokens,
                "total_tokens": ir.usage.total_tokens
            }
        })),
        "anthropic" => Ok(json!({
            "type": "message",
            "role": "assistant",
            "content": ir.message.content.iter().map(|p| match p {
                ContentPart::Text { text } => json!({"type":"text","text":text}),
                ContentPart::ToolUse { id, name, arguments } => json!({"type":"tool_use","id":id,"name":name,"input":arguments}),
                _ => json!({"type":"text","text":""})
            }).collect::<Vec<_>>(),
            "stop_reason": match ir.finish_reason {
                FinishReason::Stop => "end_turn",
                FinishReason::Length => "max_tokens",
                FinishReason::ToolCalls => "tool_use",
                FinishReason::ContentFilter => "content_filter",
                FinishReason::Other(_) => "end_turn"
            },
            "usage": {
                "input_tokens": ir.usage.prompt_tokens,
                "output_tokens": ir.usage.completion_tokens,
                "total_tokens": ir.usage.total_tokens
            }
        })),
        _ => Err(OmniError::Unsupported(format!(
            "response encoder for {}",
            target.id()
        ))),
    }
}
