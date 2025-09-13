use std::sync::Arc;

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

/// Factory for creating a bridge implementation by identifier.
pub fn create_tool_bridge(_id: &str) -> Option<Arc<dyn ToolBridge>> {
    None
}
