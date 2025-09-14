use anyhow::Result;
use clap::Parser;
use codex_tui::Cli as TuiCli;
use eframe::egui::Align;
use eframe::egui::Color32;
use eframe::egui::Frame;
use eframe::egui::Key;
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

enum Sender {
    User,
    Assistant,
}

struct Message {
    text: String,
    sender: Sender,
}

impl Message {
    fn color(&self) -> Color32 {
        match self.sender {
            Sender::User => Color32::from_rgb(80, 250, 123),
            Sender::Assistant => Color32::from_rgb(189, 147, 249),
        }
    }
}

struct Session {
    name: String,
    messages: Vec<Message>,
}

struct CodexGui {
    sessions: Vec<Session>,
    selected: usize,
    notes: Vec<String>,
    note_input: String,
    input: String,
    recording: bool,
}

impl CodexGui {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            sessions: vec![
                Session {
                    name: "Session 1".into(),
                    messages: vec![
                        Message {
                            text: "Make a calculator application in rust".into(),
                            sender: Sender::User,
                        },
                        Message {
                            text: "Sure, lets make a rust based calculator application".into(),
                            sender: Sender::Assistant,
                        },
                        Message {
                            text: "- import rt from raytrace\n- import tensor from t\n+ console.log(tensor)\n+ export default".into(),
                            sender: Sender::Assistant,
                        },
                        Message {
                            text: "Looking at http://test.com".into(),
                            sender: Sender::Assistant,
                        },
                    ],
                },
                Session {
                    name: "Session 2".into(),
                    messages: Vec::new(),
                },
            ],
            selected: 0,
            notes: vec!["Done! Test at localhost".into(), "Looking at http://test.com".into()],
            note_input: String::new(),
            input: String::new(),
            recording: false,
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
                for (i, s) in self.sessions.iter().enumerate() {
                    if ui.selectable_label(i == self.selected, &s.name).clicked() {
                        self.selected = i;
                    }
                }
                if ui.button("+ New Session").clicked() {
                    let name = format!("Session {}", self.sessions.len() + 1);
                    self.sessions.push(Session {
                        name,
                        messages: Vec::new(),
                    });
                    self.selected = self.sessions.len() - 1;
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
            ui.separator();
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.note_input);
                if ui.button("Add").clicked() && !self.note_input.trim().is_empty() {
                    self.notes.push(self.note_input.trim().to_owned());
                    self.note_input.clear();
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                for msg in &self.sessions[self.selected].messages {
                    let layout = match msg.sender {
                        Sender::User => Layout::right_to_left(Align::TOP),
                        Sender::Assistant => Layout::left_to_right(Align::TOP),
                    };
                    ui.with_layout(layout, |ui| {
                        Frame::none()
                            .fill(msg.color())
                            .rounding(8.0)
                            .inner_margin(Margin::same(8.0))
                            .show(ui, |ui| {
                                ui.label(&msg.text);
                            });
                    });
                    ui.add_space(8.0);
                }
            });
        });

        TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let send_by_enter = {
                    let response = ui.text_edit_singleline(&mut self.input);
                    response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))
                };

                if ui.button("Explain this codebase").clicked() {
                    self.send_user_message("Explain this codebase".into());
                }

                let send = ui.button("Ask").clicked() || send_by_enter;
                if send && !self.input.trim().is_empty() {
                    let text = self.input.trim().to_owned();
                    self.send_user_message(text);
                    self.input.clear();
                }

                if ui.button("Code").clicked() && !self.input.trim().is_empty() {
                    let text = format!("```\n{}\n```", self.input.trim());
                    self.send_user_message(text);
                    self.input.clear();
                }

                ui.separator();
                if self.recording {
                    if ui.button(RichText::new("⏹")).clicked() {
                        self.recording = false;
                    }
                } else if ui.button(RichText::new("🔴")).clicked() {
                    self.recording = true;
                }
            });
        });
    }
}

impl CodexGui {
    fn send_user_message(&mut self, text: String) {
        self.sessions[self.selected].messages.push(Message {
            text: text.clone(),
            sender: Sender::User,
        });
        self.sessions[self.selected].messages.push(Message {
            text: format!("Echo: {text}"),
            sender: Sender::Assistant,
        });
    }
}
