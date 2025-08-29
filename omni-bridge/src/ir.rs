use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text {
        text: String,
    },
    ImageUrl {
        url: String,
        mime: Option<String>,
    },
    // Audio, video parts can be added similarly
    ToolUse {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: Vec<ContentPart>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniMessage {
    pub role: Role,
    pub content: Vec<ContentPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniTool {
    pub name: String,
    pub description: Option<String>,
    /// JSON Schema Draft 2020-12 (store as-is, adapters downcast as needed)
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sampling {
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
    pub max_tokens: Option<u32>,
    pub presence_penalty: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub stop: Vec<String>,
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniRequest {
    pub model: String,
    pub messages: Vec<OmniMessage>,
    pub tools: Vec<OmniTool>,
    pub tool_choice: Option<ToolChoice>,
    pub response_format: Option<ResponseFormat>, // e.g., json_object, json_schema(name,schema)
    pub sampling: Sampling,
    pub metadata: serde_json::Value, // passthrough for routing hints
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseFormat {
    JsonObject,
    JsonSchema {
        name: String,
        schema: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
    Auto,
    None,
    Required,
    Named { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniResponse {
    pub message: OmniMessage, // the final assistant turn
    pub usage: Usage,
    pub finish_reason: FinishReason,
}

/// Streaming deltas normalized across providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OmniDelta {
    Text {
        text: String,
    },
    ToolUseStart {
        id: String,
        name: String,
    },
    ToolUseArgs {
        id: String,
        chunk: String,
    }, // JSON text fragments
    ToolUseEnd {
        id: String,
    },
    Finish {
        reason: FinishReason,
        usage: Option<Usage>,
    },
}
