#![cfg(not(target_os = "windows"))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use anyhow::Context;
use core_test_support::responses;
use core_test_support::test_codex_exec::test_codex_exec;
use serde_json::Value;
use wiremock::Mock;
use wiremock::ResponseTemplate;
use wiremock::http::Method;
use wiremock::matchers::method;
use wiremock::matchers::path;

use codex_lmstudio::DEFAULT_LM_STUDIO_MODEL;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exec_resolves_lmstudio_model_aliases() -> anyhow::Result<()> {
    let cases = [
        ("llama", DEFAULT_LM_STUDIO_MODEL),
        ("devstral", DEFAULT_LM_STUDIO_MODEL),
        ("qwen2", "qwen/qwen2.5-coder-14b"),
        ("qwen3", "qwen/qwen3-4b-2507"),
        ("qwen3-moe", "qwen/qwen3-coder-30b"),
        ("qwen3moe", "qwen/qwen3-coder-30b"),
        ("qwen3-moe-a3b", "qwen/qwen3-30b-a3b-2507"),
        ("qwen3 coder 30b a3b", "qwen/qwen3-30b-a3b-2507"),
        ("Qwen3 Coder 30B", "qwen/qwen3-coder-30b"),
    ];

    for (alias, expected_model) in cases {
        let test = test_codex_exec();
        let server = responses::start_mock_server().await;

        let models_payload = serde_json::json!({
            "data": [
                { "id": expected_model },
                { "id": "other/placeholder-model" }
            ]
        });

        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(models_payload.clone()))
            .expect(1)
            .mount(&server)
            .await;

        let chat_stream = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{}}]}\n\n",
            "data: [DONE]\n\n",
        );

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_raw(chat_stream, "text/event-stream"),
            )
            .expect(1)
            .mount(&server)
            .await;

        test.cmd()
            .env("CODEX_LM_STUDIO_BASE_URL", format!("{}/v1", server.uri()))
            .arg("--skip-git-repo-check")
            .arg("--backend")
            .arg("lmstudio")
            .arg("--model")
            .arg(alias)
            .arg("hi")
            .assert()
            .success();

        let requests = server
            .received_requests()
            .await
            .expect("failed to capture requests");
        let mut saw_models_check = false;
        let mut resolved_model: Option<String> = None;

        for req in &requests {
            if req.method == Method::GET && req.url.path() == "/v1/models" {
                saw_models_check = true;
            }

            if req.method == Method::POST && req.url.path() == "/v1/chat/completions" {
                let payload: Value = serde_json::from_slice(&req.body)
                    .context("LM Studio response request should be valid JSON")?;
                resolved_model = payload
                    .get("model")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
            }
        }

        assert!(
            saw_models_check,
            "alias `{alias}` did not trigger an LM Studio readiness probe"
        );

        let actual = resolved_model.context("LM Studio request missing model field")?;
        assert_eq!(
            actual, expected_model,
            "alias `{alias}` should resolve to `{expected_model}`"
        );

        server.verify().await;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exec_enables_apply_patch_tool_for_lmstudio() -> anyhow::Result<()> {
    let test = test_codex_exec();
    let server = responses::start_mock_server().await;

    let models_payload = serde_json::json!({
        "data": [
            { "id": DEFAULT_LM_STUDIO_MODEL }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(models_payload))
        .expect(1)
        .mount(&server)
        .await;

    let chat_stream = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{}}]}\n\n",
        "data: [DONE]\n\n",
    );

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(chat_stream, "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    test.cmd()
        .env("CODEX_LM_STUDIO_BASE_URL", format!("{}/v1", server.uri()))
        .arg("--skip-git-repo-check")
        .arg("--backend")
        .arg("lmstudio")
        .arg(DEFAULT_LM_STUDIO_MODEL)
        .assert()
        .success();

    let requests = server
        .received_requests()
        .await
        .expect("failed to capture requests");

    let chat_request = requests
        .iter()
        .find(|req| req.method == Method::POST && req.url.path() == "/v1/chat/completions")
        .context("LM Studio chat completion request missing")?;

    let payload: Value = serde_json::from_slice(&chat_request.body)
        .context("LM Studio chat completion request should be valid JSON")?;
    let tools = payload
        .get("tools")
        .and_then(Value::as_array)
        .context("LM Studio request missing tools array")?;

    assert!(
        tools.iter().any(|tool| {
            tool.get("type").and_then(Value::as_str) == Some("function")
                && tool
                    .get("function")
                    .and_then(|fn_value| fn_value.get("name"))
                    .and_then(Value::as_str)
                    == Some("apply_patch")
        }),
        "LM Studio chat request should include the apply_patch tool: {tools:?}"
    );

    server.verify().await;
    Ok(())
}
