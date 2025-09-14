use clap::Parser;
use codex_arg0::arg0_dispatch_or_else;
use codex_gui::Cli;
use codex_gui::run_main;

fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|codex_linux_sandbox_exe| async move {
        let cli = Cli::parse();
        run_main(cli, codex_linux_sandbox_exe).await
    })
}
