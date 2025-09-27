#![cfg(not(target_os = "windows"))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use core_test_support::responses;
use core_test_support::test_codex_exec::test_codex_exec;
use serde_json::Value;
use serde_json::json;
use wiremock::Mock;
use wiremock::ResponseTemplate;
use wiremock::http::Method;
use wiremock::matchers::any;
use wiremock::matchers::method;
use wiremock::matchers::path;

use codex_lmstudio::DEFAULT_LM_STUDIO_MODEL;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exec_sets_default_schema_for_lmstudio() -> anyhow::Result<()> {
    let test = test_codex_exec();

    let server = responses::start_mock_server().await;
    let models_payload = json!({
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
    let body = responses::sse(vec![
        serde_json::json!({
            "type": "response.created",
            "response": {"id": "resp1"}
        }),
        responses::ev_assistant_message("m1", "fixture hello"),
        responses::ev_completed("resp1"),
    ]);
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(body, "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    test.cmd()
        .env("CODEX_LM_STUDIO_BASE_URL", format!("{}/v1", server.uri()))
        .arg("--skip-git-repo-check")
        .arg("--backend")
        .arg("lmstudio")
        .arg("-m")
        .arg("llama")
        .arg("tell me a joke")
        .assert()
        .success();

    let requests = server
        .received_requests()
        .await
        .expect("failed to capture requests");
    let chat_request = requests
        .iter()
        .find(|req| req.method == Method::POST && req.url.path() == "/v1/chat/completions")
        .expect("expected LM Studio chat request");
    let payload: Value = serde_json::from_slice(&chat_request.body)?;
    let format = payload
        .get("response_format")
        .expect("request missing response_format field");

    let expected_schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Codex CLI Final Response",
        "description": "Structured JSON response emitted by Codex CLI sessions.",
        "type": "object",
        "properties": {
            "status": {
                "description": "Overall completion state.",
                "type": "string",
                "enum": ["success", "partial", "blocked", "error"]
            },
            "summary": {
                "description": "Key bullet points summarizing the work performed.",
                "type": "array",
                "items": {
                    "type": "string",
                    "minLength": 1
                },
                "minItems": 1
            },
            "testing": {
                "description": "Tests or checks that were executed.",
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "description": "Exact command that was run.",
                            "type": "string",
                            "minLength": 1
                        },
                        "status": {
                            "description": "Outcome of the command.",
                            "type": "string",
                            "enum": ["pass", "fail", "not_run", "blocked"]
                        },
                        "details": {
                            "description": "Additional context about the run.",
                            "type": "string"
                        }
                    },
                    "required": ["command", "status"],
                    "additionalProperties": false
                }
            },
            "next_steps": {
                "description": "Follow-up work that should be considered.",
                "type": "array",
                "items": {
                    "type": "string",
                    "minLength": 1
                }
            },
            "notes": {
                "description": "Extra caveats or reminders for the user.",
                "type": "array",
                "items": {
                    "type": "string"
                }
            }
        },
        "required": ["summary"],
        "additionalProperties": false
    });

    assert_eq!(
        format,
        &json!({
            "type": "json_schema",
            "json_schema": {
                "name": "codex_output_schema",
                "schema": expected_schema,
                "strict": true,
            }
        })
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exec_does_not_set_schema_for_openai() -> anyhow::Result<()> {
    let test = test_codex_exec();

    let server = responses::start_mock_server().await;
    let body = responses::sse(vec![
        serde_json::json!({
            "type": "response.created",
            "response": {"id": "resp1"}
        }),
        responses::ev_assistant_message("m1", "fixture hello"),
        responses::ev_completed("resp1"),
    ]);
    responses::mount_sse_once(&server, any(), body).await;

    test.cmd_with_server(&server)
        .arg("--skip-git-repo-check")
        .arg("-m")
        .arg("gpt-5")
        .arg("tell me a joke")
        .assert()
        .success();

    let requests = server
        .received_requests()
        .await
        .expect("failed to capture requests");
    assert_eq!(requests.len(), 1, "expected exactly one request");
    let payload: Value = serde_json::from_slice(&requests[0].body)?;
    assert!(
        payload
            .get("text")
            .and_then(|text| text.get("format"))
            .is_none(),
        "OpenAI request should not include structured output schema by default"
    );

    Ok(())
}
