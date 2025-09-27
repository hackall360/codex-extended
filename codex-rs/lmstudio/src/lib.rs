use codex_core::BUILT_IN_LM_STUDIO_MODEL_PROVIDER_ID;
use codex_core::ModelProviderInfo;
use codex_core::config::Config;
use serde_json::Value as JsonValue;
use std::io;
use std::time::Duration;

/// Default LM Studio model used when `--backend lmstudio` is specified without `--model`.
pub const DEFAULT_LM_STUDIO_MODEL: &str = "meta-llama/Meta-Llama-3.1-8B-Instruct";

const LM_STUDIO_CONNECTION_ERROR: &str = "No running LM Studio server detected. Launch LM Studio and enable the local inference server (Preferences → Developer → Enable local server).";

const SUPPORTED_ARCHITECTURES: &[&str] = &["llama", "qwen2", "qwen3", "qwen3-moe"];

const MODEL_ALIAS_TABLE: &[(&str, &str)] = &[
    ("llama", DEFAULT_LM_STUDIO_MODEL),
    ("llama3", DEFAULT_LM_STUDIO_MODEL),
    ("llama31", DEFAULT_LM_STUDIO_MODEL),
    ("llama3.1", DEFAULT_LM_STUDIO_MODEL),
    ("llama-3", DEFAULT_LM_STUDIO_MODEL),
    ("llama-31", DEFAULT_LM_STUDIO_MODEL),
    ("llama-3.1", DEFAULT_LM_STUDIO_MODEL),
    ("llama3-8b", DEFAULT_LM_STUDIO_MODEL),
    ("qwen2", "Qwen/Qwen2-7B-Instruct"),
    ("qwen-2", "Qwen/Qwen2-7B-Instruct"),
    ("qwen2-7b", "Qwen/Qwen2-7B-Instruct"),
    ("qwen3", "Qwen/Qwen3-7B-Instruct"),
    ("qwen-3", "Qwen/Qwen3-7B-Instruct"),
    ("qwen3-7b", "Qwen/Qwen3-7B-Instruct"),
    ("qwen3-moe", "Qwen/Qwen3-MoE-A2.7B-Instruct"),
    ("qwen3_moe", "Qwen/Qwen3-MoE-A2.7B-Instruct"),
    ("qwen-3-moe", "Qwen/Qwen3-MoE-A2.7B-Instruct"),
];

/// Error returned when a provided LM Studio model alias cannot be resolved.
#[derive(Debug, Clone)]
pub struct UnsupportedModelAliasError {
    alias: String,
}

impl UnsupportedModelAliasError {
    fn new(alias: impl Into<String>) -> Self {
        Self {
            alias: alias.into(),
        }
    }
}

impl std::fmt::Display for UnsupportedModelAliasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.alias.trim().is_empty() {
            write!(
                f,
                "LM Studio model name cannot be empty. Supported architectures: {}. You can also pass a full LM Studio model identifier (for example `namespace/model-name`).",
                SUPPORTED_ARCHITECTURES.join(", ")
            )
        } else {
            write!(
                f,
                "Unsupported LM Studio model alias `{}`. Supported architectures: {}. Provide one of the aliases or the full model identifier as shown in LM Studio.",
                self.alias,
                SUPPORTED_ARCHITECTURES.join(", ")
            )
        }
    }
}

impl std::error::Error for UnsupportedModelAliasError {}

/// Returns the list of LM Studio architecture aliases that Codex understands.
pub fn supported_architecture_aliases() -> &'static [&'static str] {
    SUPPORTED_ARCHITECTURES
}

/// Resolve a user-supplied model alias into the canonical LM Studio model identifier.
///
/// When `model` is `None`, the [`DEFAULT_LM_STUDIO_MODEL`] is returned.
///
/// Users may also pass fully-qualified model identifiers (as shown inside LM Studio);
/// these are returned unchanged.
pub fn resolve_model_identifier(model: Option<&str>) -> Result<String, UnsupportedModelAliasError> {
    match model {
        None => Ok(DEFAULT_LM_STUDIO_MODEL.to_string()),
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Err(UnsupportedModelAliasError::new(trimmed));
            }
            let normalized = trimmed.to_ascii_lowercase();
            if let Some((_, canonical)) = MODEL_ALIAS_TABLE
                .iter()
                .find(|(alias, _)| *alias == normalized)
            {
                return Ok((*canonical).to_string());
            }
            if trimmed.contains('/') || trimmed.contains(':') {
                return Ok(trimmed.to_string());
            }
            Err(UnsupportedModelAliasError::new(trimmed))
        }
    }
}

/// Ensure an LM Studio instance is reachable and has the configured model available locally.
///
/// This probes the provider's `/models` endpoint and confirms the requested model is present.
pub async fn ensure_lmstudio_ready(config: &Config) -> io::Result<()> {
    let provider = config
        .model_providers
        .get(BUILT_IN_LM_STUDIO_MODEL_PROVIDER_ID)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "Built-in provider `{}` not found",
                    BUILT_IN_LM_STUDIO_MODEL_PROVIDER_ID
                ),
            )
        })?;

    probe_server(provider, &config.model).await
}

async fn probe_server(provider: &ModelProviderInfo, model: &str) -> io::Result<()> {
    let base_url = provider.base_url.as_ref().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "LM Studio provider missing base_url",
        )
    })?;

    // LM Studio exposes an OpenAI-compatible API rooted at `/v1`.
    let models_url = format!("{}/models", base_url.trim_end_matches('/'));

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let response = client.get(&models_url).send().await.map_err(|err| {
        tracing::warn!("Failed to connect to LM Studio server: {err:?}");
        io::Error::other(LM_STUDIO_CONNECTION_ERROR)
    })?;

    if !response.status().is_success() {
        tracing::warn!(
            "LM Studio `/models` request failed: HTTP {}",
            response.status()
        );
        return Err(io::Error::other(LM_STUDIO_CONNECTION_ERROR));
    }

    let payload = response
        .json::<JsonValue>()
        .await
        .map_err(|err| io::Error::other(format!("Failed to parse LM Studio response: {err}")))?;

    if !model_available(&payload, model) {
        return Err(io::Error::other(format!(
            "LM Studio does not have a model named `{model}`. Download the requested architecture in LM Studio or pass a fully-qualified model identifier."
        )));
    }

    Ok(())
}

fn model_available(payload: &JsonValue, target_model: &str) -> bool {
    fn matches_entry(entry: &JsonValue, target: &str) -> bool {
        let normalized_target = target.trim().to_ascii_lowercase();
        let short_target = target
            .trim()
            .rsplit('/')
            .next()
            .map(str::to_ascii_lowercase)
            .unwrap_or_else(|| normalized_target.clone());

        let check = |candidate: &str| {
            let normalized_candidate = candidate.trim().to_ascii_lowercase();
            normalized_candidate == normalized_target
                || normalized_candidate == short_target
                || normalized_candidate.ends_with(&short_target)
        };

        entry
            .get("id")
            .and_then(|v| v.as_str())
            .map(check)
            .or_else(|| entry.get("name").and_then(|v| v.as_str()).map(check))
            .or_else(|| entry.get("model").and_then(|v| v.as_str()).map(check))
            .or_else(|| entry.as_str().map(check))
            .unwrap_or(false)
    }

    if let Some(entries) = payload.get("data").and_then(|v| v.as_array()) {
        if entries
            .iter()
            .any(|entry| matches_entry(entry, target_model))
        {
            return true;
        }
    }

    if let Some(entries) = payload.get("models").and_then(|v| v.as_array()) {
        if entries
            .iter()
            .any(|entry| matches_entry(entry, target_model))
        {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;
    use wiremock::Mock;
    use wiremock::MockServer;
    use wiremock::ResponseTemplate;
    use wiremock::matchers::method;
    use wiremock::matchers::path;

    use codex_core::config::Config;
    use codex_core::config::ConfigOverrides;
    use codex_core::config::ConfigToml;

    #[test]
    fn resolves_aliases_to_canonical_models() {
        assert_eq!(
            resolve_model_identifier(Some("llama")).unwrap(),
            DEFAULT_LM_STUDIO_MODEL
        );
        assert_eq!(
            resolve_model_identifier(Some("qwen2")).unwrap(),
            "Qwen/Qwen2-7B-Instruct"
        );
        assert_eq!(
            resolve_model_identifier(Some("qwen3")).unwrap(),
            "Qwen/Qwen3-7B-Instruct"
        );
        assert_eq!(
            resolve_model_identifier(Some("qwen3-moe")).unwrap(),
            "Qwen/Qwen3-MoE-A2.7B-Instruct"
        );
    }

    #[test]
    fn returns_default_model_when_none_is_provided() {
        assert_eq!(
            resolve_model_identifier(None).unwrap(),
            DEFAULT_LM_STUDIO_MODEL
        );
    }

    #[test]
    fn rejects_unknown_aliases() {
        let err = resolve_model_identifier(Some("unknown")).unwrap_err();
        assert!(
            err.to_string()
                .contains("Supported architectures: llama, qwen2, qwen3, qwen3-moe")
        );
    }

    #[tokio::test]
    async fn ensure_ready_checks_for_available_model() {
        let server = MockServer::start().await;
        let response = serde_json::json!({
            "data": [
                { "id": DEFAULT_LM_STUDIO_MODEL },
                { "id": "Qwen/Qwen3-7B-Instruct" }
            ]
        });
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&server)
            .await;

        let codex_home = TempDir::new().expect("tempdir");
        let config_toml = ConfigToml::default();
        let overrides = ConfigOverrides {
            model: Some(DEFAULT_LM_STUDIO_MODEL.to_string()),
            model_provider: Some(BUILT_IN_LM_STUDIO_MODEL_PROVIDER_ID.to_string()),
            ..ConfigOverrides::default()
        };
        let mut config = Config::load_from_base_config_with_overrides(
            config_toml,
            overrides,
            codex_home.path().to_path_buf(),
        )
        .expect("load config");

        let base_url = format!("{}/v1", server.uri());
        if let Some(provider) = config
            .model_providers
            .get_mut(BUILT_IN_LM_STUDIO_MODEL_PROVIDER_ID)
        {
            provider.base_url = Some(base_url.clone());
        }
        if config
            .model_provider_id
            .eq(BUILT_IN_LM_STUDIO_MODEL_PROVIDER_ID)
        {
            config.model_provider.base_url = Some(base_url);
        }

        ensure_lmstudio_ready(&config)
            .await
            .expect("lm studio ready");
    }

    #[tokio::test]
    async fn ensure_ready_errors_when_model_missing() {
        let server = MockServer::start().await;
        let response = serde_json::json!({
            "data": [ { "id": "some/other-model" } ]
        });
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&server)
            .await;

        let codex_home = TempDir::new().expect("tempdir");
        let config_toml = ConfigToml::default();
        let overrides = ConfigOverrides {
            model: Some(DEFAULT_LM_STUDIO_MODEL.to_string()),
            model_provider: Some(BUILT_IN_LM_STUDIO_MODEL_PROVIDER_ID.to_string()),
            ..ConfigOverrides::default()
        };
        let mut config = Config::load_from_base_config_with_overrides(
            config_toml,
            overrides,
            codex_home.path().to_path_buf(),
        )
        .expect("load config");

        let base_url = format!("{}/v1", server.uri());
        if let Some(provider) = config
            .model_providers
            .get_mut(BUILT_IN_LM_STUDIO_MODEL_PROVIDER_ID)
        {
            provider.base_url = Some(base_url.clone());
        }
        if config
            .model_provider_id
            .eq(BUILT_IN_LM_STUDIO_MODEL_PROVIDER_ID)
        {
            config.model_provider.base_url = Some(base_url);
        }

        let err = ensure_lmstudio_ready(&config)
            .await
            .expect_err("missing model");
        assert!(err.to_string().contains("does not have a model"));
    }
}
