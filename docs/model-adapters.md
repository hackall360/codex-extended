# Model adapter layer

Codex normally speaks to OpenAI compatible APIs.  To enable full "bring your own model"
support, the `codex-core` crate now exposes a `ModelAdapter` trait that translates
Codex prompts into provider specific requests and normalises the streamed responses.

A provider using a non-standard protocol can set `wire_api = "custom"` in its
`ModelProviderInfo` configuration and supply an adapter when constructing the
`ModelClient`:

```rust
use std::sync::Arc;
use codex_core::{ModelClient, ModelAdapter, OpenAiChatAdapter, WireApi};

// Register adapter (OpenAI chat adapter shown for brevity).
let adapter = Arc::new(OpenAiChatAdapter);
let client = ModelClient::new_with_adapter(
    config,
    auth_manager,
    provider, // provider.wire_api == WireApi::Custom
    effort,
    summary,
    session_id,
    Some(adapter),
);
```

Custom backends implement the trait and can freely call local processes or
third‑party services without requiring model fine tuning.
