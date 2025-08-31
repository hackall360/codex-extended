use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Configuration options loaded from `config.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Config {
    /// URL of the backend service.
    pub backend_url: String,
    /// Name of the chat model to use.
    pub chat_model: String,
    /// Name of the embedding model to use.
    pub embedding_model: String,
    /// Vector store implementation to use.
    pub store: StoreChoice,
    /// Location for on-disk data.
    pub data_path: PathBuf,
    /// Port the server should bind to.
    pub port: u16,
    /// Optional RESP port for exposing raw Redis protocol.
    pub resp_port: Option<u16>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            backend_url: "http://localhost:8000".to_string(),
            chat_model: "gpt-4o".to_string(),
            embedding_model: "nomic-embed-text".to_string(),
            store: StoreChoice::Memory,
            data_path: PathBuf::from("./data"),
            port: 0,
            resp_port: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum StoreChoice {
    #[default]
    Memory,
    Redis,
}

#[derive(Debug, Deserialize, Default)]
struct PartialConfig {
    backend_url: Option<String>,
    chat_model: Option<String>,
    embedding_model: Option<String>,
    store: Option<StoreChoice>,
    data_path: Option<PathBuf>,
    port: Option<u16>,
    resp_port: Option<u16>,
}

/// Errors that can occur while loading configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("invalid model '{0}'")]
    InvalidModel(String),
}

impl Config {
    /// Load configuration from disk. The path can be overridden with the
    /// `CODEX_CONFIG` environment variable. When the file is missing, default
    /// values are used.
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = std::env::var("CODEX_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                Path::new(&home).join(".codex/config.toml")
            });

        let contents = match fs::read_to_string(&config_path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(ConfigError::Io(e)),
        };

        let partial: PartialConfig = if contents.trim().is_empty() {
            PartialConfig::default()
        } else {
            toml::from_str(&contents)?
        };

        let mut cfg = Config::default();
        if let Some(url) = partial.backend_url {
            cfg.backend_url = url;
        }
        if let Some(model) = partial.chat_model {
            validate_model(&model)?;
            cfg.chat_model = model;
        }
        if let Some(model) = partial.embedding_model {
            validate_model(&model)?;
            cfg.embedding_model = model;
        }
        if let Some(store) = partial.store {
            cfg.store = store;
        }
        if let Some(path) = partial.data_path {
            cfg.data_path = path;
        }
        if let Some(port) = partial.port {
            cfg.port = port;
        }
        if let Some(resp) = partial.resp_port {
            cfg.resp_port = Some(resp);
        }

        Ok(cfg)
    }
}

fn validate_model(name: &str) -> Result<(), ConfigError> {
    const CHAT: &[&str] = &["gpt-4o", "llama3"];
    const EMBED: &[&str] = &["nomic-embed-text", "all-minilm"];
    if CHAT.contains(&name) || EMBED.contains(&name) {
        Ok(())
    } else {
        Err(ConfigError::InvalidModel(name.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_missing_uses_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        unsafe {
            std::env::set_var("CODEX_CONFIG", &path);
        }
        let cfg = Config::load().unwrap();
        assert_eq!(cfg, Config::default());
    }

    #[test]
    fn invalid_model_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "chat_model = \"bad-model\"").unwrap();
        unsafe {
            std::env::set_var("CODEX_CONFIG", &path);
        }
        let err = Config::load().unwrap_err();
        match err {
            ConfigError::InvalidModel(m) => assert_eq!(m, "bad-model"),
            _ => panic!("unexpected error: {err:?}"),
        }
    }
}
