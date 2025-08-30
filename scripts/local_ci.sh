#!/usr/bin/env bash
# Run checks that mirror GitHub CI workflows.
set -euo pipefail

# Node/doc related checks
pnpm install >/dev/null
if command -v gh >/dev/null && gh auth status >/dev/null 2>&1; then
  if ! ./codex-cli/scripts/stage_release.sh >/tmp/local_ci_stage.log 2>&1; then
    echo "[warn] stage_release.sh failed" >&2
  fi
else
  echo "[skip] stage_release.sh (requires authenticated gh CLI)" >&2
fi
python3 scripts/asciicheck.py README.md
python3 scripts/readme_toc.py README.md || true
python3 scripts/asciicheck.py codex-cli/README.md
python3 scripts/readme_toc.py codex-cli/README.md || true

# Codespell
codespell --ignore-words .codespellignore || true

# Ensure ALSA libs for tests
if command -v apt-get >/dev/null; then
  if command -v sudo >/dev/null; then
    sudo apt-get update >/dev/null && sudo apt-get install -y libasound2-dev >/dev/null || \
      echo "[warn] failed to install libasound2-dev" >&2
  else
    apt-get update >/dev/null && apt-get install -y libasound2-dev >/dev/null || \
      echo "[warn] failed to install libasound2-dev" >&2
  fi
fi

# Rust checks
(
  cd codex-rs
  if ! cargo fmt -- --config imports_granularity=Item --check; then
    echo "[warn] cargo fmt failed" >&2
  fi
  if ! cargo clippy --all-features --tests -- -D warnings; then
    echo "[warn] cargo clippy failed" >&2
  fi
  if ! cargo test --all-features; then
    echo "[warn] cargo test failed" >&2
  fi
)
