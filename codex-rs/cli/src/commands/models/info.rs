use codex_common::CliConfigOverrides;
use codex_core::config::{Config, ConfigOverrides};

pub fn run(cli_config_overrides: CliConfigOverrides) -> ! {
    let cli_overrides = match cli_config_overrides.parse_overrides() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error parsing -c overrides: {e}");
            std::process::exit(1);
        }
    };

    let config = match Config::load_with_cli_overrides(cli_overrides, ConfigOverrides::default()) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error loading configuration: {e}");
            std::process::exit(1);
        }
    };

    println!("model: {}", config.model);
    println!("provider: {}", config.model_provider_id);
    println!(
        "context_window: {}",
        config
            .model_context_window
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );
    println!(
        "max_output_tokens: {}",
        config
            .model_max_output_tokens
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );
    println!("capabilities:");
    println!(
        "  needs_special_apply_patch_instructions: {}",
        config.model_family.needs_special_apply_patch_instructions
    );
    println!(
        "  supports_reasoning_summaries: {}",
        config.model_family.supports_reasoning_summaries
    );
    println!(
        "  uses_local_shell_tool: {}",
        config.model_family.uses_local_shell_tool
    );
    println!(
        "  apply_patch_tool_type: {}",
        match config.model_family.apply_patch_tool_type {
            Some(t) => format!("{t:?}"),
            None => "None".to_string(),
        }
    );

    std::process::exit(0);
}
