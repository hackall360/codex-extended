mod client;
mod parser;
mod pull;
mod url;

pub use client::OllamaClient;
use codex_core::config::Config;
use codex_core::model_family::{ModelFamily, built_in_model_capabilities, find_family_for_model};
pub use pull::CliProgressReporter;
pub use pull::PullEvent;
pub use pull::PullProgressReporter;
pub use pull::TuiProgressReporter;

/// Default OSS model to use when `--oss` is passed without an explicit `-m`.
pub const DEFAULT_OSS_MODEL: &str = "gpt-oss:20b";

/// Prepare the local OSS environment when `--oss` is selected.
///
/// - Ensures a local Ollama server is reachable.
/// - Checks if the model exists locally and pulls it if missing.
pub async fn ensure_oss_ready(config: &mut Config) -> std::io::Result<()> {
    let requested_model = config.model.clone();

    // Verify local Ollama is reachable.
    let ollama_client = crate::OllamaClient::try_from_oss_provider(config).await?;

    match ollama_client.fetch_models().await {
        Ok(models) => {
            if !models.iter().any(|m| m == &requested_model) {
                if requested_model == DEFAULT_OSS_MODEL
                    && let Some(first) = models.first()
                {
                    tracing::info!("Using local Ollama model `{}`", first);
                    config.model = first.clone();
                    config.model_family =
                        find_family_for_model(first, built_in_model_capabilities()).unwrap_or_else(
                            || ModelFamily {
                                slug: first.clone(),
                                family: first.clone(),
                                needs_special_apply_patch_instructions: false,
                                supports_reasoning_summaries: false,
                                uses_local_shell_tool: false,
                                apply_patch_tool_type: None,
                            },
                        );
                } else {
                    let mut reporter = crate::CliProgressReporter::new();
                    ollama_client
                        .pull_with_reporter(&requested_model, &mut reporter)
                        .await?;
                }
            }
        }
        Err(err) => {
            tracing::warn!("Failed to query local models from Ollama: {}.", err);
        }
    }

    if config.model_context_window.is_none() || config.model_max_output_tokens.is_none() {
        if let Ok(Some(meta)) = ollama_client.fetch_model_metadata(&config.model).await {
            if config.model_context_window.is_none() {
                config.model_context_window = meta.context_window;
            }
            if config.model_max_output_tokens.is_none() {
                config.model_max_output_tokens = meta.max_output_tokens;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_core::BUILT_IN_OSS_MODEL_PROVIDER_ID;
    use codex_core::config::Config;
    use codex_core::config::ConfigOverrides;
    use codex_core::config::ConfigToml;

    // Skip network tests when sandbox networking is disabled.
    fn networking_disabled() -> bool {
        std::env::var(codex_core::spawn::CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR).is_ok()
    }

    #[tokio::test]
    async fn test_ensure_oss_ready_uses_existing_model() {
        if networking_disabled() {
            tracing::info!(
                "{} set; skipping test_ensure_oss_ready_uses_existing_model",
                codex_core::spawn::CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR
            );
            return;
        }

        let server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/v1/models"))
            .respond_with(wiremock::ResponseTemplate::new(200))
            .mount(&server)
            .await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/api/tags"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_raw(
                    serde_json::json!({
                        "models": [{"name": "llama3"}],
                    })
                    .to_string(),
                    "application/json",
                ),
            )
            .mount(&server)
            .await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/api/pull"))
            .respond_with(wiremock::ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;

        let overrides = ConfigOverrides {
            model: Some(DEFAULT_OSS_MODEL.to_string()),
            cwd: None,
            approval_policy: None,
            edit_mode: None,
            sandbox_mode: None,
            model_provider: Some(BUILT_IN_OSS_MODEL_PROVIDER_ID.to_string()),
            config_profile: None,
            codex_linux_sandbox_exe: None,
            base_instructions: None,
            include_plan_tool: None,
            include_apply_patch_tool: None,
            include_view_image_tool: None,
            disable_response_storage: None,
            show_raw_agent_reasoning: None,
            tools_web_search_request: None,
        };

        let tmp = tempfile::tempdir().expect("tempdir");
        let mut config = Config::load_from_base_config_with_overrides(
            ConfigToml::default(),
            overrides,
            tmp.path().to_path_buf(),
        )
        .expect("config");

        let provider =
            codex_core::create_oss_provider_with_base_url(&format!("{}/v1", server.uri()));
        config.model_provider = provider.clone();
        config
            .model_providers
            .insert(BUILT_IN_OSS_MODEL_PROVIDER_ID.to_string(), provider);

        ensure_oss_ready(&mut config).await.expect("ensure ready");
        assert_eq!(config.model, "llama3");
        assert_eq!(config.model_family.slug, "llama3");
    }

    #[tokio::test]
    async fn test_ensure_oss_ready_pulls_default() {
        if networking_disabled() {
            tracing::info!(
                "{} set; skipping test_ensure_oss_ready_pulls_default",
                codex_core::spawn::CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR
            );
            return;
        }

        let server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/v1/models"))
            .respond_with(wiremock::ResponseTemplate::new(200))
            .mount(&server)
            .await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/api/tags"))
            .respond_with(wiremock::ResponseTemplate::new(200))
            .mount(&server)
            .await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/api/pull"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_raw("{\"status\":\"success\"}\n", "application/json"),
            )
            .mount(&server)
            .await;

        let overrides = ConfigOverrides {
            model: Some(DEFAULT_OSS_MODEL.to_string()),
            cwd: None,
            approval_policy: None,
            edit_mode: None,
            sandbox_mode: None,
            model_provider: Some(BUILT_IN_OSS_MODEL_PROVIDER_ID.to_string()),
            config_profile: None,
            codex_linux_sandbox_exe: None,
            base_instructions: None,
            include_plan_tool: None,
            include_apply_patch_tool: None,
            include_view_image_tool: None,
            disable_response_storage: None,
            show_raw_agent_reasoning: None,
            tools_web_search_request: None,
        };

        let tmp = tempfile::tempdir().expect("tempdir");
        let mut config = Config::load_from_base_config_with_overrides(
            ConfigToml::default(),
            overrides,
            tmp.path().to_path_buf(),
        )
        .expect("config");

        let provider =
            codex_core::create_oss_provider_with_base_url(&format!("{}/v1", server.uri()));
        config.model_provider = provider.clone();
        config
            .model_providers
            .insert(BUILT_IN_OSS_MODEL_PROVIDER_ID.to_string(), provider);

        ensure_oss_ready(&mut config).await.expect("ensure ready");
        assert_eq!(config.model, DEFAULT_OSS_MODEL);
    }

    #[tokio::test]
    async fn test_ensure_oss_ready_populates_model_metadata() {
        if networking_disabled() {
            tracing::info!(
                "{} set; skipping test_ensure_oss_ready_populates_model_metadata",
                codex_core::spawn::CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR
            );
            return;
        }

        let server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/v1/models"))
            .respond_with(wiremock::ResponseTemplate::new(200))
            .mount(&server)
            .await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/api/tags"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_raw(
                    serde_json::json!({
                        "models": [{"name": "llama3"}],
                    })
                    .to_string(),
                    "application/json",
                ),
            )
            .mount(&server)
            .await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/api/pull"))
            .respond_with(wiremock::ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/api/show/llama3"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_raw(
                    serde_json::json!({
                        "details": {"parameters": {"num_ctx": 1234}},
                        "num_predict": 5678,
                    })
                    .to_string(),
                    "application/json",
                ),
            )
            .mount(&server)
            .await;

        let overrides = ConfigOverrides {
            model: Some("llama3".to_string()),
            cwd: None,
            approval_policy: None,
            edit_mode: None,
            sandbox_mode: None,
            model_provider: Some(BUILT_IN_OSS_MODEL_PROVIDER_ID.to_string()),
            config_profile: None,
            codex_linux_sandbox_exe: None,
            base_instructions: None,
            include_plan_tool: None,
            include_apply_patch_tool: None,
            include_view_image_tool: None,
            disable_response_storage: None,
            show_raw_agent_reasoning: None,
            tools_web_search_request: None,
        };

        let tmp = tempfile::tempdir().expect("tempdir");
        let mut config = Config::load_from_base_config_with_overrides(
            ConfigToml::default(),
            overrides,
            tmp.path().to_path_buf(),
        )
        .expect("config");

        let provider =
            codex_core::create_oss_provider_with_base_url(&format!("{}/v1", server.uri()));
        config.model_provider = provider.clone();
        config
            .model_providers
            .insert(BUILT_IN_OSS_MODEL_PROVIDER_ID.to_string(), provider);

        ensure_oss_ready(&mut config).await.expect("ensure ready");
        assert_eq!(config.model_context_window, Some(1234));
        assert_eq!(config.model_max_output_tokens, Some(5678));
    }
}
