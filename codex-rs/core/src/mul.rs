#[cfg(feature = "mul")]
use codex_mul::{
    adapter::{JsonAdapter, MulAdapter},
    error::Result as MulResult,
    langs::{python, rust},
};
#[cfg(feature = "mul")]
use codex_protocol::models::{ContentItem, ResponseItem};

#[cfg(feature = "mul")]
#[derive(Debug, Clone, Copy)]
pub enum MulLanguage {
    Json,
    Python,
    Rust,
}

#[cfg(feature = "mul")]
pub fn encode(item: &mut ResponseItem, lang: MulLanguage) -> MulResult<()> {
    if let ResponseItem::Message { content, .. } = item {
        for ci in content.iter_mut() {
            if let ContentItem::InputText { text } = ci {
                let program = match lang {
                    MulLanguage::Json => JsonAdapter::from_source(text)?,
                    MulLanguage::Python => python::Adapter::from_source(text)?,
                    MulLanguage::Rust => rust::Adapter::from_source(text)?,
                };
                *text = JsonAdapter::to_source(&program)?;
            }
        }
    }
    Ok(())
}

#[cfg(feature = "mul")]
pub fn decode(item: &mut ResponseItem, lang: MulLanguage) -> MulResult<()> {
    if let ResponseItem::Message { content, .. } = item {
        for ci in content.iter_mut() {
            if let ContentItem::OutputText { text } = ci {
                let program = JsonAdapter::from_source(text)?;
                *text = match lang {
                    MulLanguage::Json => JsonAdapter::to_source(&program)?,
                    MulLanguage::Python => python::Adapter::to_source(&program)?,
                    MulLanguage::Rust => rust::Adapter::to_source(&program)?,
                };
            }
        }
    }
    Ok(())
}
