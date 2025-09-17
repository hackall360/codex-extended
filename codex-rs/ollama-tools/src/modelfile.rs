use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "codex-ollama-modelfile",
    about = "Generate a Codex-tuned Ollama Modelfile from an existing model"
)]
struct Cli {
    /// Base model slug to derive from (e.g., qwen2.5-coder:7b)
    #[arg(long)]
    model: String,

    /// Output path for Modelfile (directory or file). Defaults to ./Modelfile
    #[arg(long)]
    out: Option<PathBuf>,

    /// Path to the `ollama` executable (defaults to `ollama` in PATH)
    #[arg(long)]
    ollama_bin: Option<PathBuf>,

    /// Context window tokens (default 131072)
    #[arg(long, default_value_t = 131_072)]
    num_ctx: u32,

    /// Temperature (default 0.2)
    #[arg(long, default_value_t = 0.2)]
    temperature: f32,

    /// Top-p (default 0.9)
    #[arg(long, default_value_t = 0.9)]
    top_p: f32,

    /// Seed (default 7)
    #[arg(long, default_value_t = 7)]
    seed: u64,

    /// num_predict cap per turn (default 4096)
    #[arg(long, default_value_t = 4096)]
    num_predict: u32,
}

fn codex_system_prompt() -> String {
    let prompt = r#"You are Codex’s local agent. Return ONLY JSON. Never include prose, code fences, XML/HTML wrappers, or surrounding text.

When you need to act, ALWAYS return exactly:
{"type":"tool","name":"<tool>","input":{...}}

When you need to reply to the user, return exactly:
{"type":"message","content":"<text>"}

Allowed tools (use one per turn):
- shell: input = {"command":["<exe>","arg1",...], "workdir"?: string, "timeout_ms"?: number}
- apply_patch: input = {"input":"*** Begin Patch\n*** Add File: path/to/file\n+<contents>\n*** End Patch"}

Guidelines:
- One tool call per turn.
- Do not invent tool names. Use only shell and apply_patch.
- Prefer apply_patch for file edits (full patch envelope).
- Prefer shell for safe commands (e.g., python tictactoe.py --help).
- Keep changes within the working directory. Avoid destructive commands."#;
    prompt.to_string()
}

fn fetch_original_modelfile(ollama_bin: &Path, model: &str) -> Result<Option<String>> {
    let output = Command::new(ollama_bin)
        .args(["show", model, "--modelfile"])
        .output()
        .context("failed to run `ollama show --modelfile`")?;
    if !output.status.success() {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(text))
    }
}

fn synthesize_modelfile(model: &str, base: Option<&str>, cfg: &Cli) -> String {
    let mut out = String::new();
    let header_present = base
        .map(|b| b.contains("\nFROM ") || b.starts_with("FROM "))
        .unwrap_or(false);
    if let Some(orig) = base {
        // Include original content first.
        out.push_str(orig.trim_end());
        out.push_str("\n\n");
        if !header_present {
            out.push_str(&format!("FROM {model}\n\n"));
        }
    } else {
        out.push_str(&format!("FROM {model}\n\n"));
    }

    // Append Codex parameters and system prompt.
    out.push_str(&format!("PARAMETER num_ctx {}\n", cfg.num_ctx));
    out.push_str(&format!("PARAMETER temperature {}\n", cfg.temperature));
    out.push_str(&format!("PARAMETER top_p {}\n", cfg.top_p));
    out.push_str(&format!("PARAMETER seed {}\n", cfg.seed));
    out.push_str(&format!("PARAMETER num_predict {}\n\n", cfg.num_predict));
    out.push_str("SYSTEM \"\"\"\n");
    out.push_str(&codex_system_prompt());
    out.push_str("\n\"\"\"\n");
    out
}

fn write_output(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create dir {}", parent.display()))?;
    }
    fs::write(path, content).with_context(|| format!("write {}", path.display()))
}

fn resolve_out_path(out: Option<PathBuf>) -> PathBuf {
    match out {
        Some(p) => {
            if p.is_dir() || (!p.as_path().exists() && p.extension().is_none()) {
                p.join("Modelfile")
            } else {
                p
            }
        }
        None => PathBuf::from("Modelfile"),
    }
}

fn find_ollama_bin(cli: &Cli) -> PathBuf {
    if let Some(p) = &cli.ollama_bin {
        return p.clone();
    }
    // Default to `ollama` in PATH
    PathBuf::from("ollama")
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let out_path = resolve_out_path(cli.out.clone());
    let ollama_bin = find_ollama_bin(&cli);

    let original = fetch_original_modelfile(&ollama_bin, &cli.model)?;
    let content = synthesize_modelfile(&cli.model, original.as_deref(), &cli);
    write_output(&out_path, &content)?;
    eprintln!(
        "Wrote Codex-tuned Modelfile for {} to {}",
        cli.model,
        out_path.display()
    );
    Ok(())
}
