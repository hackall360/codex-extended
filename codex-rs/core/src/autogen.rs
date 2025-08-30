use std::process::Command;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use serde_json::json;

use crate::ModelProviderInfo;
use crate::create_oss_provider_with_base_url;

/// Integration helpers for invoking Python's AutoGen framework.
///
/// AutoGen (https://github.com/microsoft/autogen) enables building
/// multi-agent applications in Python. These helpers execute small
/// Python snippets so that core consumers can delegate work to AutoGen
/// without relying on the plugin system.
pub fn is_available() -> bool {
    // Try to import the `autogen` module. If this succeeds, AutoGen is
    // available in the current Python environment.
    let script = "import importlib.util, sys\n".to_string()
        + "mod = importlib.util.find_spec('autogen')\n"
        + "sys.exit(0 if mod is not None else 1)";

    Command::new("python")
        .arg("-c")
        .arg(script)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Run a simple AutoGen conversation using the provided `prompt`.
///
/// This requires the `autogen` Python package as well as any
/// configuration needed for the default LLM used by AutoGen. On
/// success, the stdout of the Python process is returned.
pub fn run_autogen_prompt(prompt: &str) -> Result<String> {
    let script = format!(
        r#"
from autogen import AssistantAgent, UserProxyAgent, GroupChat, GroupChatManager

assistant = AssistantAgent('assistant')
user = UserProxyAgent('user')

manager = GroupChatManager(GroupChat(agents=[assistant, user]))
user.initiate_chat(manager, message={prompt:?})
"#,
    );

    let output = Command::new("python")
        .arg("-c")
        .arg(script)
        .output()
        .context("failed to run AutoGen Python script")?;

    if !output.status.success() {
        return Err(anyhow!(
            "AutoGen execution failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Run an AutoGen conversation against the specified `provider` and `model`.
///
/// The provider configuration is translated into AutoGen's `config_list`
/// format so that callers can leverage non-default model backends such as
/// Azure OpenAI or local OSS servers.
pub fn run_autogen_with_provider(
    prompt: &str,
    provider: &ModelProviderInfo,
    model: &str,
) -> Result<String> {
    let mut entry = json!({ "model": model });

    if let Some(base_url) = &provider.base_url {
        entry["base_url"] = json!(base_url);
    }

    if let Some(env_key) = &provider.env_key
        && let Ok(value) = std::env::var(env_key)
    {
        entry["api_key"] = json!(value);
    }

    let config_list = json!([entry]);

    let script = format!(
        r#"
from autogen import AssistantAgent, UserProxyAgent, GroupChat, GroupChatManager

config_list = {config_list}

assistant = AssistantAgent('assistant', llm_config={{"config_list": config_list}})
user = UserProxyAgent('user', llm_config={{"config_list": config_list}})

manager = GroupChatManager(GroupChat(agents=[assistant, user]))
user.initiate_chat(manager, message={prompt:?})
"#,
        config_list = config_list,
        prompt = prompt,
    );

    let output = Command::new("python")
        .arg("-c")
        .arg(script)
        .output()
        .context("failed to run AutoGen Python script")?;

    if !output.status.success() {
        return Err(anyhow!(
            "AutoGen execution failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Convenience wrapper for running AutoGen against a local model server.
///
/// `base_url` should point to an OpenAI-compatible endpoint exposing the
/// desired `model`.
pub fn run_autogen_local_prompt(prompt: &str, base_url: &str, model: &str) -> Result<String> {
    let provider = create_oss_provider_with_base_url(base_url);
    run_autogen_with_provider(prompt, &provider, model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_check_does_not_panic() {
        // We don't assert on availability because CI environments may not
        // have AutoGen installed. The goal is simply to ensure the helper
        // executes without panicking.
        let _ = is_available();
    }

    #[test]
    fn run_with_provider_returns_error_when_autogen_missing() {
        let provider = crate::create_oss_provider_with_base_url("http://localhost:11434/v1");
        let result = run_autogen_with_provider("hi", &provider, "llama2");
        assert!(result.is_err());
    }
}
