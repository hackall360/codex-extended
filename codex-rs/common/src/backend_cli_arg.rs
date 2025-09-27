use clap::ValueEnum;

/// CLI flag values for selecting the Codex runtime backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum BackendCliArg {
    /// Use the default OpenAI backend.
    Openai,
    /// Use the bundled open-source Ollama integration.
    Oss,
    /// Use a local LM Studio instance.
    Lmstudio,
}

impl BackendCliArg {
    /// Returns the model provider key associated with this backend, if any.
    pub fn provider_key(self) -> Option<&'static str> {
        match self {
            BackendCliArg::Openai => None,
            BackendCliArg::Oss => Some(codex_core::BUILT_IN_OSS_MODEL_PROVIDER_ID),
            BackendCliArg::Lmstudio => Some(codex_core::BUILT_IN_LM_STUDIO_MODEL_PROVIDER_ID),
        }
    }

    pub fn is_oss(self) -> bool {
        matches!(self, BackendCliArg::Oss)
    }

    pub fn is_lmstudio(self) -> bool {
        matches!(self, BackendCliArg::Lmstudio)
    }

    pub fn is_local(self) -> bool {
        matches!(self, BackendCliArg::Oss | BackendCliArg::Lmstudio)
    }
}
