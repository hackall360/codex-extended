mod ir;
pub use ir::*;
mod caps;
pub use caps::*;
mod error;
pub use error::*;
pub mod adapters;
pub mod stream;

use serde_json::Value;

#[async_trait::async_trait]
pub trait Adapter: Send + Sync {
    /// Stable identifier: "openai", "anthropic", "gemini", …
    fn id(&self) -> &'static str;
    fn capabilities(&self) -> Caps;

    /// Decode a native provider request JSON into IR.
    fn decode_request(&self, native: &Value) -> Result<OmniRequest, OmniError>;

    /// Encode an IR request into the provider's native request JSON.
    fn encode_request(&self, ir: &OmniRequest) -> Result<Value, OmniError>;

    /// Decode a provider response JSON into IR (non-streaming).
    fn decode_response(&self, native: &Value) -> Result<OmniResponse, OmniError>;

    /// Streaming: feed raw bytes/chunks (SSE/event JSON) → zero or more IR deltas.
    fn stream_decode(&self, chunk: &[u8]) -> Result<Vec<OmniDelta>, OmniError>;

    /// Streaming: encode IR deltas into provider-native stream chunk(s).
    fn stream_encode(&self, deltas: &[OmniDelta]) -> Result<Vec<Vec<u8>>, OmniError>;
}

pub struct Bridge {
    adapters: std::collections::HashMap<&'static str, std::sync::Arc<dyn Adapter>>,
}

impl Bridge {
    pub fn new() -> Self {
        Self {
            adapters: Default::default(),
        }
    }
    pub fn with_adapter(mut self, a: std::sync::Arc<dyn Adapter>) -> Self {
        self.adapters.insert(a.id(), a);
        self
    }

    pub fn adapter(&self, id: &str) -> Option<std::sync::Arc<dyn Adapter>> {
        self.adapters.get(id).cloned()
    }

    /// Translate a native request of one provider into the native request of another.
    pub fn translate_request(
        &self,
        from: &str,
        to: &str,
        native_req: &Value,
    ) -> Result<Value, OmniError> {
        let a = self
            .adapter(from)
            .ok_or_else(|| OmniError::Unsupported(format!("unknown provider {from}")))?;
        let b = self
            .adapter(to)
            .ok_or_else(|| OmniError::Unsupported(format!("unknown provider {to}")))?;
        let ir = a.decode_request(native_req)?;
        b.encode_request(&ir)
    }

    /// Translate a native response the other way.
    pub fn translate_response(
        &self,
        from: &str,
        to: &str,
        native_resp: &Value,
    ) -> Result<Value, OmniError> {
        let a = self
            .adapter(from)
            .ok_or_else(|| OmniError::Unsupported(format!("unknown provider {from}")))?;
        let b = self
            .adapter(to)
            .ok_or_else(|| OmniError::Unsupported(format!("unknown provider {to}")))?;
        let ir = a.decode_response(native_resp)?;
        adapters::encode_response_native(b.as_ref(), &ir)
    }
}
