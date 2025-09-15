use std::sync::Arc;

use codex_core::ResponseEvent;
use codex_core::ResponseItem;
use codex_core::spawn::CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR;
use codex_ollama::OllamaBackend;
use codex_ollama::OllamaToolBridge;
use pretty_assertions::assert_eq;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

#[tokio::test]
async fn sends_json_prompt_and_parses_tool_call() {
    if std::env::var(CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR).is_ok() {
        println!(
            "Skipping test because it cannot execute when network is disabled in a Codex sandbox.",
        );
        return;
    }

    let server = MockServer::start().await;

    let tool_json = "{\"type\":\"tool\",\"name\":\"t\",\"input\":{}}";
    let template = ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "model": "m",
        "created_at": "2024-01-01T00:00:00Z",
        "response": tool_json,
        "done": true
    }));

    Mock::given(method("POST"))
        .and(path("/api/generate"))
        .respond_with(template)
        .expect(1)
        .mount(&server)
        .await;

    let backend = OllamaBackend::with_tool_bridge(&server.uri(), Arc::new(OllamaToolBridge))
        .expect("backend");

    let events = backend.chat("m", "hi").await.expect("chat");

    let requests = server.received_requests().await.expect("requests");
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).expect("json");
    let prompt = body["prompt"].as_str().expect("prompt str");
    assert!(prompt.contains("Respond only with JSON"));

    match &events[0] {
        ResponseEvent::OutputItemDone(ResponseItem::CustomToolCall { name, input, .. }) => {
            assert_eq!(name, "t");
            assert_eq!(input, "{}");
        }
        other => panic!("unexpected event: {other:?}"),
    }
}
