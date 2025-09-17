use clap::Parser;
use codex_arg0::arg0_dispatch_or_else;
use codex_common::CliConfigOverrides;
use codex_tui::Cli;
use codex_tui::run_main;

#[derive(Parser, Debug)]
struct TopCli {
    #[clap(flatten)]
    config_overrides: CliConfigOverrides,

    #[clap(flatten)]
    inner: Cli,
}

fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|codex_linux_sandbox_exe| async move {
        // Register the Ollama tooling bridge so any provider whose id starts
        // with "ollama" can bridge JSON outputs into tool calls.
        #[allow(clippy::disallowed_methods)]
        {
            codex_ollama::register_ollama_tool_bridge();
        }
        let top_cli = TopCli::parse();
        let mut inner = top_cli.inner;
        inner
            .config_overrides
            .raw_overrides
            .splice(0..0, top_cli.config_overrides.raw_overrides);
        let usage = run_main(inner, codex_linux_sandbox_exe).await?;
        if !usage.is_zero() {
            println!("{}", codex_core::protocol::FinalOutput::from(usage));
        }
        Ok(())
    })
}
