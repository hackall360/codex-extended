use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;

use crate::client_common::Prompt;
use crate::client_common::ResponseEvent;
use crate::error::Result;

/// Bridge that can adapt prompts and parse model outputs for tool usage.
pub trait ToolBridge: Send + Sync + std::fmt::Debug {
    /// Inject provider-specific instructions into the prompt prior to dispatch.
    fn encode_prompt(&self, prompt: &mut Prompt);

    /// Parse a raw model `ResponseEvent` into zero or more standard events.
    fn decode_event(&self, event: ResponseEvent) -> Result<Vec<ResponseEvent>>;
}

/// Factory type that produces a [`ToolBridge`].
pub type ToolBridgeFactory = fn() -> Arc<dyn ToolBridge>;

static REGISTRY: OnceLock<Mutex<HashMap<String, ToolBridgeFactory>>> = OnceLock::new();

/// Register a [`ToolBridgeFactory`] under `id`.
pub fn register_tool_bridge(id: &str, factory: ToolBridgeFactory) {
    let map = REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    map.lock().unwrap().insert(id.to_string(), factory);
}

/// Factory for creating a bridge implementation by identifier.
pub fn create_tool_bridge(id: &str) -> Option<Arc<dyn ToolBridge>> {
    let map = REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    map.lock().unwrap().get(id).map(|f| f())
}
