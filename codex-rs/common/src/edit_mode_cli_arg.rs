//! CLI parser for edit mode (request | block | trusted).

use clap::ValueEnum;

use codex_core::config_types::EditMode;

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum EditModeCliArg {
    Request,
    Block,
    Trusted,
}

impl From<EditModeCliArg> for EditMode {
    fn from(value: EditModeCliArg) -> Self {
        match value {
            EditModeCliArg::Request => EditMode::Request,
            EditModeCliArg::Block => EditMode::Block,
            EditModeCliArg::Trusted => EditMode::Trusted,
        }
    }
}
