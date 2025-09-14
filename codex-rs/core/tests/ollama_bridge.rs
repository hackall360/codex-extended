#![expect(clippy::expect_used)]

use std::sync::Arc;

use codex_core::ContentItem;
use codex_core::ModelClient;
use codex_core::ModelProviderInfo;
use codex_core::Prompt;
use codex_core::ResponseEvent;
use codex_core::ResponseItem;
use codex_core::built_in_model_providers;
use codex_core::spawn::CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR;
use codex_ollama::register_ollama_tool_bridge;
use codex_protocol::mcp_protocol::ConversationId;
use core_test_support::load_default_config_for_test;
use futures::StreamExt;
use pretty_assertions::assert_eq;
use tempfile::TempDir;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

fn network_disabled() -> bool {
    std::env::var(CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR).is_ok()
}

fn sse_from_content(content: &str) -> String {
    let first = serde_json::json!({
        "choices": [{ "delta": { "content": content } }]
    });
    let second = serde_json::json!({
        "choices": [{ "delta": {} }]
    });
    format!("data: {first}\n\ndata: {second}\n\ndata: [DONE]\n\n")
}

async fn run_client(
    provider: ModelProviderInfo,
    sse_body: &str,
) -> (Vec<ResponseEvent>, serde_json::Value) {
    let server = MockServer::start().await;

    let template = ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_raw(sse_body.to_string(), "text/event-stream");

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(template)
        .expect(1)
        .mount(&server)
        .await;

    let mut provider = provider;
    provider.base_url = Some(format!("{}/v1", server.uri()));

    let codex_home = TempDir::new().expect("tempdir");
    let mut config = load_default_config_for_test(&codex_home);
    config.model_provider_id = provider.name.clone();
    config.model_provider = provider.clone();
    let effort = config.model_reasoning_effort;
    let summary = config.model_reasoning_summary;
    let config = Arc::new(config);

    let client = ModelClient::new(
        Arc::clone(&config),
        None,
        provider,
        effort,
        summary,
        ConversationId::new(),
    );

    let mut prompt = Prompt::default();
    prompt.input = vec![ResponseItem::Message {
        id: None,
        role: "user".into(),
        content: vec![ContentItem::InputText { text: "hi".into() }],
    }];

    let mut stream = client.stream(&prompt).await.expect("stream");
    let mut events = Vec::new();
    while let Some(ev) = stream.next().await {
        events.push(ev.expect("event"));
    }

    let requests = server.received_requests().await.expect("requests");
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).expect("request json");

    (events, body)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn injects_system_prompt_and_parses_message() {
    if network_disabled() {
        println!(
            "Skipping test because it cannot execute when network is disabled in a Codex sandbox.",
        );
        return;
    }

    register_ollama_tool_bridge();

    let providers = built_in_model_providers();
    let provider = providers["ollama"].clone();
    let sse = sse_from_content(include_str!("fixtures/ollama_message.json"));
    let (events, body) = run_client(provider, &sse).await;

    assert_eq!(body["messages"][0]["role"], "system");

    assert!(events.iter().any(|ev| matches!(
        ev,
        ResponseEvent::OutputItemDone(ResponseItem::Message { .. })
    )));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn custom_tool_call_routes_via_mcp_shell() {
    if network_disabled() {
        println!(
            "Skipping test because it cannot execute when network is disabled in a Codex sandbox.",
        );
        return;
    }

    register_ollama_tool_bridge();
    let providers = built_in_model_providers();

    let fixtures = [
        (
            "ollama-llama",
            include_str!("fixtures/ollama_llama_tool.json"),
            "llama",
        ),
        (
            "ollama-granite",
            include_str!("fixtures/ollama_granite_tool.json"),
            "granite",
        ),
        (
            "ollama-cogito",
            include_str!("fixtures/ollama_cogito_tool.json"),
            "cogito",
        ),
    ];

    for (id, content, expected) in fixtures {
        let provider = providers[id].clone();
        let sse = sse_from_content(content);
        let (events, _) = run_client(provider, &sse).await;
        let item = events
            .into_iter()
            .find_map(|ev| match ev {
                ResponseEvent::OutputItemDone(ri) => Some(ri),
                _ => None,
            })
            .expect("output item");
        match item {
            ResponseItem::CustomToolCall { name, input, .. } => {
                assert_eq!(name, "local_shell__exec");
                let v: serde_json::Value = serde_json::from_str(&input).expect("json");
                assert_eq!(v["command"][1], expected);
            }
            other => panic!("expected custom tool call, got {other:?}"),
        }
    }
}
