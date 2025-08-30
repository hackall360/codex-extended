# Repository Guidelines

## Project Structure & Module Organization
- `codex-rs/`: Rust workspace (core agent, tools, TUI, CLI). Key crates: `core` (business logic), `tui` (terminal UI), `cli` (binary), plus utilities.
- `codex-cli/`: Node.js packaging/scripts for distribution.
- `web-ui/`: Flutter/Dart frontend experiments.
- `docs/`: Design notes and contributor docs.
- Plugin system has been removed; integrate additional tools directly into the core or via configured MCP servers.

## Build, Test, and Development Commands
- Rust (from `codex-rs/`):
  - `cargo build -p codex-cli` — build CLI (debug).
  - `cargo build -p codex-cli --release` — optimized build.
  - `cargo test -p codex-core` — core tests; run workspace with `cargo test --all-features`.
  - `just fmt` — format Rust code; `just fix -p <crate>` — clippy autofix.
- Node (from `codex-cli/`):
  - `pnpm i` then `pnpm run build` — install and build.
- Flutter (from `web-ui/`):
  - `flutter pub get` then `flutter build web` (optional experiments).

## Coding Style & Naming Conventions
- Rust: `rustfmt`, `clippy`. Prefer explicit names; avoid one-letter vars. Follow TUI style helpers (`codex-rs/tui/styles.md`) and use ratatui’s `Stylize` (e.g., "OK".green()).
- JS/TS: Prettier config at repo root.
- Dart: `dart format` defaults.
- Crates are prefixed with `codex-` (e.g., `codex-core`).

## Testing Guidelines
- Rust uses `cargo test` throughout; the TUI uses `insta` snapshots:
  - `cargo test -p codex-tui`
  - Review with `cargo insta pending-snapshots -p codex-tui` and accept via `cargo insta accept -p codex-tui`.
- Keep tests small, deterministic, and colocated under each crate.

## Commit & Pull Request Guidelines
- Prefer Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`, etc.); concise, imperative subject, contextual body.
- PRs should include: summary, rationale, screenshots/recordings for UI, and linked issues.
- Keep patches focused; update docs when behavior or UX changes.

## Security & Configuration Tips
- Sandboxing/approvals are controlled by `config.toml` (see `codex-rs/config.md`). For fully unattended workflows, set `autonomous_mode = true`.
