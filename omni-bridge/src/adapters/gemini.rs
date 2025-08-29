use crate::*;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct GeminiAdapter;
impl GeminiAdapter {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

#[async_trait]
impl Adapter for GeminiAdapter {
    fn id(&self) -> &'static str {
        "gemini"
    }
    fn capabilities(&self) -> Caps {
        Caps::empty()
    }
    fn decode_request(&self, _native: &Value) -> Result<OmniRequest, OmniError> {
        Err(OmniError::Unsupported(
            "gemini decode not implemented".into(),
        ))
    }
    fn encode_request(&self, _ir: &OmniRequest) -> Result<Value, OmniError> {
        Err(OmniError::Unsupported(
            "gemini encode not implemented".into(),
        ))
    }
    fn decode_response(&self, _native: &Value) -> Result<OmniResponse, OmniError> {
        Err(OmniError::Unsupported(
            "gemini decode not implemented".into(),
        ))
    }
    fn stream_decode(&self, _chunk: &[u8]) -> Result<Vec<OmniDelta>, OmniError> {
        Ok(vec![])
    }
    fn stream_encode(&self, _deltas: &[OmniDelta]) -> Result<Vec<Vec<u8>>, OmniError> {
        Ok(vec![])
    }
}
