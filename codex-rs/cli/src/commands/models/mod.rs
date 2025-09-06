use clap::{Parser, Subcommand};
use codex_common::CliConfigOverrides;

pub mod info;

#[derive(Debug, Parser)]
pub struct ModelsCli {
    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    #[command(subcommand)]
    pub command: ModelsSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ModelsSubcommand {
    /// Show details about the active model.
    Info,
}

pub fn run(cli: ModelsCli) -> ! {
    match cli.command {
        ModelsSubcommand::Info => info::run(cli.config_overrides),
    }
}
