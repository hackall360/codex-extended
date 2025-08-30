#!/usr/bin/env bash
# Run checks that mirror GitHub CI workflows.
set -euo pipefail

# Node/doc related checks
pnpm install >/dev/null
if ! ./codex-cli/scripts/stage_release.sh >/tmp/local_ci_stage.log 2>&1; then
  echo "[warn] stage_release.sh failed (requires gh auth?)" >&2
fi
python3 scripts/asciicheck.py README.md
python3 scripts/readme_toc.py README.md || true
python3 scripts/asciicheck.py codex-cli/README.md
python3 scripts/readme_toc.py codex-cli/README.md || true

# Codespell
codespell --ignore-words .codespellignore || true

# Rust checks
(
  cd codex-rs
  cargo fmt -- --config imports_granularity=Item --check
  cargo clippy --all-features --tests -- -D warnings
  cargo test --all-features
)
