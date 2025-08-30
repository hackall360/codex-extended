use async_trait::async_trait;

use crate::client_common::Prompt;
use crate::client_common::ResponseStream;
use crate::error::Result;
use crate::model_family::ModelFamily;
use crate::model_provider_info::ModelProviderInfo;

/// Abstraction over different LLM backends. Implementations translate a Codex
/// [`Prompt`] into provider specific requests and normalise the streaming
/// responses back into [`ResponseStream`].
#[async_trait]
pub trait ModelAdapter: Send + Sync + std::fmt::Debug {
    /// Open a streaming connection for the given `prompt` using the provided
    /// HTTP `client` and model `provider` definition.
    async fn stream(
        &self,
        prompt: &Prompt,
        model_family: &ModelFamily,
        client: &reqwest::Client,
        provider: &ModelProviderInfo,
    ) -> Result<ResponseStream>;
}

/// Default adapter for OpenAI compatible Chat Completions endpoints.
#[derive(Debug)]
pub struct OpenAiChatAdapter;

#[async_trait]
impl ModelAdapter for OpenAiChatAdapter {
    async fn stream(
        &self,
        prompt: &Prompt,
        model_family: &ModelFamily,
        client: &reqwest::Client,
        provider: &ModelProviderInfo,
    ) -> Result<ResponseStream> {
        crate::chat_completions::stream_chat_completions(prompt, model_family, client, provider)
            .await
    }
}
