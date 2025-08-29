use crate::{OmniDelta, OmniError};

/// Generic SSE helpers. For now these are simple placeholders; provider-specific
/// adapters implement their own streaming logic.
pub fn decode_placeholder(_chunk: &[u8]) -> Result<Vec<OmniDelta>, OmniError> {
    Ok(vec![])
}

pub fn encode_placeholder(_deltas: &[OmniDelta]) -> Result<Vec<Vec<u8>>, OmniError> {
    Ok(vec![])
}
