use anyhow::Result;
use clap::CommandFactory;
use clap::Parser;
use clap_complete::Shell;
use clap_complete::generate;
use codex_chatgpt::apply_command::ApplyCommand;
use codex_chatgpt::apply_command::run_apply_command;
use codex_cli::LandlockCommand;
use codex_cli::SeatbeltCommand;
use codex_cli::login::run_login_status;
use codex_cli::login::run_login_with_api_key;
use codex_cli::login::run_login_with_chatgpt;
use codex_cli::login::run_logout;
use codex_cli::proto;
use codex_cli::proto::ProtoCli;
use codex_common::CliConfigOverrides;
use codex_core::protocol::FinalOutput;
use codex_exec::Cli as ExecCli;
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
use std::mem;
use std::path::PathBuf;

/// Command line interface for the graphical Codex client.
///
/// If no subcommand is specified, options will be forwarded to the interactive UI.
#[derive(Debug, Parser)]
#[clap(author, version, subcommand_negates_reqs = true, bin_name = "codex")]
pub struct Cli {
    #[clap(flatten)]
    pub config_overrides: CliConfigOverrides,

    /// Launch the terminal UI instead of the graphical interface.
    #[arg(long = "tui-mode", default_value_t = false)]
    pub tui_mode: bool,

    #[clap(flatten)]
    interactive: TuiCli,

    #[clap(subcommand)]
    subcommand: Option<Subcommand>,
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Run Codex non-interactively.
    #[clap(visible_alias = "e")]
    Exec(ExecCli),

    /// Manage login.
    Login(LoginCommand),

    /// Remove stored authentication credentials.
    Logout(LogoutCommand),

    /// Experimental: run Codex as an MCP server.
    Mcp,

    /// Run the Protocol stream via stdin/stdout.
    #[clap(visible_alias = "p")]
    Proto(ProtoCli),

    /// Generate shell completion scripts.
    Completion(CompletionCommand),

    /// Internal debugging commands.
    Debug(DebugArgs),

    /// Apply the latest diff produced by Codex agent as a `git apply` to your local working tree.
    #[clap(visible_alias = "a")]
    Apply(ApplyCommand),

    /// Internal: generate TypeScript protocol bindings.
    #[clap(hide = true)]
    GenerateTs(GenerateTsCommand),
}

#[derive(Debug, Parser)]
struct CompletionCommand {
    /// Shell to generate completions for
    #[clap(value_enum, default_value_t = Shell::Bash)]
    shell: Shell,
}

#[derive(Debug, Parser)]
struct DebugArgs {
    #[command(subcommand)]
    cmd: DebugCommand,
}

#[derive(Debug, clap::Subcommand)]
enum DebugCommand {
    /// Run a command under Seatbelt (macOS only).
    Seatbelt(SeatbeltCommand),

    /// Run a command under Landlock+seccomp (Linux only).
    Landlock(LandlockCommand),
}

#[derive(Debug, Parser)]
struct LoginCommand {
    #[clap(skip)]
    config_overrides: CliConfigOverrides,

    #[arg(long = "api-key", value_name = "API_KEY")]
    api_key: Option<String>,

    #[command(subcommand)]
    action: Option<LoginSubcommand>,
}

#[derive(Debug, clap::Subcommand)]
enum LoginSubcommand {
    /// Show login status.
    Status,
}

#[derive(Debug, Parser)]
struct LogoutCommand {
    #[clap(skip)]
    config_overrides: CliConfigOverrides,
}

#[derive(Debug, Parser)]
struct GenerateTsCommand {
    /// Output directory where .ts files will be written
    #[arg(short = 'o', long = "out", value_name = "DIR")]
    out_dir: PathBuf,

    /// Optional path to the Prettier executable to format generated files
    #[arg(short = 'p', long = "prettier", value_name = "PRETTIER_BIN")]
    prettier: Option<PathBuf>,
}

/// Entry point for the graphical Codex client.
pub async fn run_main(cli: Cli, codex_linux_sandbox_exe: Option<PathBuf>) -> Result<()> {
    match cli.subcommand {
        None => {
            let mut tui_cli = cli.interactive;
            prepend_config_flags(&mut tui_cli.config_overrides, cli.config_overrides);
            if cli.tui_mode {
                let usage = codex_tui::run_main(tui_cli, codex_linux_sandbox_exe).await?;
                if !usage.is_zero() {
                    println!("{}", FinalOutput::from(usage));
                }
            } else if let Err(err) = run_gui() {
                eprintln!("failed to start GUI: {err}");
            }
        }
        Some(Subcommand::Exec(mut exec_cli)) => {
            prepend_config_flags(&mut exec_cli.config_overrides, cli.config_overrides);
            codex_exec::run_main(exec_cli, codex_linux_sandbox_exe).await?;
        }
        Some(Subcommand::Mcp) => {
            codex_mcp_server::run_main(codex_linux_sandbox_exe, cli.config_overrides).await?;
        }
        Some(Subcommand::Login(mut login_cli)) => {
            prepend_config_flags(&mut login_cli.config_overrides, cli.config_overrides);
            match login_cli.action {
                Some(LoginSubcommand::Status) => {
                    run_login_status(login_cli.config_overrides).await;
                }
                None => {
                    if let Some(api_key) = login_cli.api_key {
                        run_login_with_api_key(login_cli.config_overrides, api_key).await;
                    } else {
                        run_login_with_chatgpt(login_cli.config_overrides).await;
                    }
                }
            }
        }
        Some(Subcommand::Logout(mut logout_cli)) => {
            prepend_config_flags(&mut logout_cli.config_overrides, cli.config_overrides);
            run_logout(logout_cli.config_overrides).await;
        }
        Some(Subcommand::Proto(mut proto_cli)) => {
            prepend_config_flags(&mut proto_cli.config_overrides, cli.config_overrides);
            proto::run_main(proto_cli).await?;
        }
        Some(Subcommand::Completion(completion_cli)) => {
            print_completion(completion_cli);
        }
        Some(Subcommand::Debug(debug_args)) => match debug_args.cmd {
            DebugCommand::Seatbelt(mut seatbelt_cli) => {
                prepend_config_flags(&mut seatbelt_cli.config_overrides, cli.config_overrides);
                codex_cli::debug_sandbox::run_command_under_seatbelt(
                    seatbelt_cli,
                    codex_linux_sandbox_exe,
                )
                .await?;
            }
            DebugCommand::Landlock(mut landlock_cli) => {
                prepend_config_flags(&mut landlock_cli.config_overrides, cli.config_overrides);
                codex_cli::debug_sandbox::run_command_under_landlock(
                    landlock_cli,
                    codex_linux_sandbox_exe,
                )
                .await?;
            }
        },
        Some(Subcommand::Apply(mut apply_cli)) => {
            prepend_config_flags(&mut apply_cli.config_overrides, cli.config_overrides);
            run_apply_command(apply_cli, None).await?;
        }
        Some(Subcommand::GenerateTs(gen_cli)) => {
            codex_protocol_ts::generate_ts(&gen_cli.out_dir, gen_cli.prettier.as_deref())?;
        }
    }

    Ok(())
}

/// Prepend root-level overrides so they have lower precedence than
/// CLI-specific ones specified after the subcommand (if any).
fn prepend_config_flags(
    subcommand_config_overrides: &mut CliConfigOverrides,
    cli_config_overrides: CliConfigOverrides,
) {
    subcommand_config_overrides
        .raw_overrides
        .splice(0..0, cli_config_overrides.raw_overrides);
}

fn print_completion(cmd: CompletionCommand) {
    let mut app = Cli::command();
    let name = "codex";
    generate(cmd.shell, &mut app, name, &mut std::io::stdout());
}

fn run_gui() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native("Codex", options, Box::new(|cc| Box::new(CodexGui::new(cc))))
}

struct Message {
    text: String,
    color: Color32,
}

#[derive(Default)]
struct CodexGui {
    sessions: Vec<String>,
    notes: Vec<String>,
    messages: Vec<Message>,
    input: String,
    new_session: String,
    new_note: String,
    prompt: String,
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
            new_session: String::new(),
            new_note: String::new(),
            prompt: "Make a calculator application in rust".into(),
        }
    }

    fn add_session(&mut self) {
        if !self.new_session.is_empty() {
            self.sessions.push(mem::take(&mut self.new_session));
        }
    }

    fn add_note(&mut self) {
        if !self.new_note.is_empty() {
            self.notes.push(mem::take(&mut self.new_note));
        }
    }

    fn send_message(&mut self, color: Color32) {
        if !self.input.is_empty() {
            self.prompt = self.input.clone();
            self.messages.push(Message {
                text: mem::take(&mut self.input),
                color,
            });
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
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.new_session);
                    if ui.button("+").clicked() {
                        self.add_session();
                    }
                });
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
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.new_note);
                if ui.button("+").clicked() {
                    self.add_note();
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                Frame::none()
                    .fill(Color32::from_rgb(166, 226, 46))
                    .rounding(4.0)
                    .inner_margin(Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.label(&self.prompt);
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
                if ui.button("Explain this codebase").clicked() {
                    self.send_message(Color32::from_rgb(139, 233, 253));
                }
                if ui.button("Ask").clicked() {
                    self.send_message(Color32::from_rgb(189, 147, 249));
                }
                if ui.button("Code").clicked() {
                    self.send_message(Color32::from_rgb(80, 250, 123));
                }
                ui.separator();
                let _ = ui.button(RichText::new("🔴"));
                let _ = ui.button(RichText::new("⏹"));
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn add_session_appends() {
        let mut gui = CodexGui {
            new_session: "S3".into(),
            ..Default::default()
        };
        gui.add_session();
        assert_eq!(gui.sessions.len(), 1);
        assert_eq!(gui.sessions[0], "S3");
    }

    #[test]
    fn send_message_updates_state() {
        let mut gui = CodexGui {
            input: "hi".into(),
            ..Default::default()
        };
        gui.send_message(Color32::from_rgb(1, 2, 3));
        assert_eq!(gui.prompt, "hi");
        assert_eq!(gui.messages.last().unwrap().text, "hi");
    }
}
