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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exec_resolves_lmstudio_model_aliases() -> anyhow::Result<()> {
    let cases = [
        ("llama", "meta-llama/Meta-Llama-3.1-8B-Instruct"),
        ("qwen2", "Qwen/Qwen2-7B-Instruct"),
        ("qwen3", "Qwen/Qwen3-7B-Instruct"),
        ("qwen3-moe", "Qwen/Qwen3-MoE-A2.7B-Instruct"),
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
