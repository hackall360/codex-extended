use std::path::Path;

use codex_mcp_server::CodexToolCallParam;
use mcp_test_support::{create_mock_chat_completions_server, McpProcess};
use mcp_types::{JSONRPCNotification, JSONRPCResponse, RequestId};
use pretty_assertions::assert_eq;
use serde_json::json;
use tempfile::TempDir;
use tokio::time::timeout;

const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[tokio::test]
async fn test_streaming_deltas_forwarded() {
    let responses = vec![create_streaming_response()];
    let server = create_mock_chat_completions_server(responses).await;

    let codex_home = TempDir::new().expect("create temp dir");
    create_config_toml(codex_home.path(), &server.uri()).expect("write config");

    let mut mcp = McpProcess::new(codex_home.path())
        .await
        .expect("spawn mcp process");
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize())
        .await
        .expect("init timed out")
        .expect("init failed");

    let codex_request_id = mcp
        .send_codex_tool_call(CodexToolCallParam {
            prompt: "say hello".to_string(),
            ..Default::default()
        })
        .await
        .expect("send codex tool call");

    let reasoning_delta: JSONRPCNotification = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("codex/event/agent_reasoning_delta"),
    )
    .await
    .expect("reasoning delta timeout")
    .expect("reasoning delta notif");
    let params = reasoning_delta.params.expect("params");
    assert_eq!(
        params.get("_meta").and_then(|m| m.get("requestId")),
        Some(&json!(codex_request_id)),
    );
    assert_eq!(
        params
            .get("msg")
            .and_then(|m| m.get("delta"))
            .and_then(|d| d.as_str()),
        Some("thinking"),
    );

    let msg_delta1: JSONRPCNotification = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("codex/event/agent_message_delta"),
    )
    .await
    .expect("msg delta1 timeout")
    .expect("msg delta1 notif");
    let params = msg_delta1.params.expect("params");
    assert_eq!(
        params
            .get("msg")
            .and_then(|m| m.get("delta"))
            .and_then(|d| d.as_str()),
        Some("hel"),
    );

    let msg_delta2: JSONRPCNotification = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("codex/event/agent_message_delta"),
    )
    .await
    .expect("msg delta2 timeout")
    .expect("msg delta2 notif");
    let params = msg_delta2.params.expect("params");
    assert_eq!(
        params
            .get("msg")
            .and_then(|m| m.get("delta"))
            .and_then(|d| d.as_str()),
        Some("lo"),
    );

    let final_msg: JSONRPCNotification = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("codex/event/agent_message"),
    )
    .await
    .expect("final msg timeout")
    .expect("final msg notif");
    let params = final_msg.params.expect("params");
    assert_eq!(
        params
            .get("msg")
            .and_then(|m| m.get("message"))
            .and_then(|d| d.as_str()),
        Some("hello"),
    );

    let response: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(codex_request_id)),
    )
    .await
    .expect("call response timeout")
    .expect("call response");
    assert_eq!(
        response.result,
        json!({
            "content": [{ "type": "text", "text": "hello" }]
        }),
    );
}

fn create_streaming_response() -> String {
    let events = vec![
        json!({"choices":[{"delta":{"reasoning":{"text":"thinking"}}}]}),
        json!({"choices":[{"delta":{"content":"hel"}}]}),
        json!({"choices":[{"delta":{"content":"lo"},"finish_reason":"stop"}]}),
    ];
    let mut sse = String::new();
    for ev in events {
        sse.push_str("data: ");
        sse.push_str(&serde_json::to_string(&ev).expect("serialize"));
        sse.push_str("\n\n");
    }
    sse.push_str("data: DONE\n\n");
    sse
}

fn create_config_toml(codex_home: &Path, server_uri: &str) -> std::io::Result<()> {
    let config_toml = codex_home.join("config.toml");
    std::fs::write(
        config_toml,
        format!(
            r#"
model = "mock-model"
approval_policy = "never"
sandbox_mode = "danger-full-access"

model_provider = "mock_provider"

[model_providers.mock_provider]
name = "Mock provider for test"
base_url = "{server_uri}/v1"
wire_api = "chat"
request_max_retries = 0
stream_max_retries = 0
"#
        ),
    )
}
