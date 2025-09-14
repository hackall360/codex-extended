use anyhow::Result;
use clap::Parser;
use codex_tui::Cli as TuiCli;
use eframe::egui::Align;
use eframe::egui::Color32;
use eframe::egui::Frame;
use eframe::egui::Layout;
use eframe::egui::Margin;
use eframe::egui::RichText;
use eframe::egui::ScrollArea;
use eframe::egui::SidePanel;
use eframe::egui::TopBottomPanel;
use eframe::egui::{self};
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

struct Message {
    text: String,
    color: Color32,
}

struct CodexGui {
    sessions: Vec<String>,
    notes: Vec<String>,
    messages: Vec<Message>,
    input: String,
}

impl CodexGui {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            sessions: vec!["Session 1".into(), "Session 2".into()],
            notes: vec!["Done! Test at localhost".into(), "Looking at http://test.com".into()],
            messages: vec![
                Message {
                    text: "Sure, lets make a rust based calculator application".into(),
                    color: Color32::from_rgb(189, 147, 249),
                },
                Message {
                    text: "- import rt from raytrace\n- import tensor from t\n+ console.log(tensor)\n+ export default".into(),
                    color: Color32::from_rgb(80, 250, 123),
                },
                Message {
                    text: "Looking at http://test.com".into(),
                    color: Color32::from_rgb(139, 233, 253),
                },
            ],
            input: String::new(),
        }
    }
}

impl eframe::App for CodexGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        TopBottomPanel::top("top").show(ctx, |ui| {
            ui.heading("Codex GUI 0.0.1");
        });

        SidePanel::left("sessions")
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("Sessions");
                ui.separator();
                for s in &self.sessions {
                    ui.label(s);
                }
            });

        SidePanel::right("notes").resizable(false).show(ctx, |ui| {
            ui.heading("Notes");
            ui.separator();
            let mut remove: Option<usize> = None;
            for (i, note) in self.notes.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(note);
                    if ui.button(RichText::new("✖").color(Color32::RED)).clicked() {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                self.notes.remove(i);
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                Frame::none()
                    .fill(Color32::from_rgb(166, 226, 46))
                    .rounding(4.0)
                    .inner_margin(Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.label("Make a calculator application in rust");
                    });
            });
            ui.add_space(10.0);
            ScrollArea::vertical().show(ui, |ui| {
                for msg in &self.messages {
                    Frame::none()
                        .fill(msg.color)
                        .rounding(8.0)
                        .inner_margin(Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.label(&msg.text);
                        });
                    ui.add_space(8.0);
                }
            });
        });

        TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.input);
                ui.button("Explain this codebase").clicked();
                ui.button("Ask").clicked();
                ui.button("Code").clicked();
                ui.separator();
                let _ = ui.button(RichText::new("🔴"));
                let _ = ui.button(RichText::new("⏹"));
            });
        });
    }
}
