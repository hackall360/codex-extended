use anyhow::Result;
use clap::Parser;
use codex_tui::Cli as TuiCli;
use eframe::egui;
use std::path::PathBuf;

/// Command line interface for the graphical Codex client.
#[derive(Debug, Parser)]
#[command(version)]
pub struct Cli {
    /// Launch the terminal UI instead of the graphical interface.
    #[arg(long = "tui-mode", default_value_t = false)]
    pub tui_mode: bool,

    #[clap(flatten)]
    pub tui: TuiCli,
}

/// Entry point for the graphical Codex client.
pub async fn run_main(cli: Cli, codex_linux_sandbox_exe: Option<PathBuf>) -> Result<()> {
    if cli.tui_mode {
        let _ = codex_tui::run_main(cli.tui, codex_linux_sandbox_exe).await?;
    } else if let Err(err) = run_gui() {
        eprintln!("failed to start GUI: {err}");
    }
    Ok(())
}

fn run_gui() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native("Codex", options, Box::new(|cc| Box::new(CodexGui::new(cc))))
}

struct CodexGui;

impl CodexGui {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self
    }
}

impl eframe::App for CodexGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Codex GUI");
            ui.label("GUI mode is under construction.");
        });
    }
}
