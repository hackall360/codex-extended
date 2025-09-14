use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;

use crate::client_common::Prompt;
use crate::client_common::ResponseEvent;
use crate::error::Result;

/// JSON schema describing the structured response format required when
/// bridging model output into Codex `ResponseItem`s or tool calls.
pub const TOOLING_SCHEMA: &str = r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Tooling output",
  "type": "object",
  "required": ["type"],
  "properties": {
    "type": { "type": "string", "enum": ["tool", "message"] },
    "name": { "type": "string" },
    "input": {},
    "content": { "type": "string" }
  },
  "allOf": [
    {
      "if": { "properties": { "type": { "const": "tool" } } },
      "then": { "required": ["name", "input"], "not": { "required": ["content"] } }
    },
    {
      "if": { "properties": { "type": { "const": "message" } } },
      "then": { "required": ["content"], "not": { "anyOf": [{ "required": ["name"] }, { "required": ["input"] }] } }
    }
  ]
}"#;

/// Bridge that can adapt prompts and parse model outputs for tool usage.
pub trait ToolingBridge: Send + Sync + std::fmt::Debug {
    /// Inject provider-specific schema instructions into the prompt prior to dispatch.
    fn wrap_prompt(&self, prompt: &mut Prompt);

    /// Parse a raw model `ResponseEvent` into zero or more standard events.
    fn parse_event(&self, event: ResponseEvent) -> Result<Vec<ResponseEvent>>;
}

/// Factory type that produces a [`ToolingBridge`].
pub type ToolingBridgeFactory = fn() -> Arc<dyn ToolingBridge>;

static REGISTRY: OnceLock<Mutex<HashMap<String, ToolingBridgeFactory>>> = OnceLock::new();

/// Register a [`ToolingBridgeFactory`] under `id`.
pub fn register_tooling_bridge(id: &str, factory: ToolingBridgeFactory) {
    let map = REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut map) = map.lock() {
        map.insert(id.to_string(), factory);
    }
}

/// Factory for creating a bridge implementation by identifier.
pub fn create_tooling_bridge(id: &str) -> Option<Arc<dyn ToolingBridge>> {
    let map = REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    map.lock().ok().and_then(|m| m.get(id).map(|f| f()))
}
