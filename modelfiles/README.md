Codex‑tuned Ollama Modelfiles

This folder contains Modelfiles you can build locally with `ollama create` to tune models for Codex’s JSON tool contract. These Modelfiles:

- Add a compact system prompt that enforces JSON‑only output and the exact tool shapes Codex expects.
- Nudge sampling parameters for structured output stability (low temperature, moderate top_p, fixed seed).
- Increase context window to 128k tokens via `PARAMETER num_ctx 131072` (subject to your hardware and model limits).

Build commands

- Cogito 3B (Codex‑tuned):
  - `ollama create -f modelfiles/cogito3bcodex/Modelfile cogito-3b-codex`

- Qwen2.5‑Coder 7B (Codex‑tuned):
  - `ollama create -f modelfiles/qwen2.5coder7bcodex/Modelfile qwen2.5-coder-7b-codex`

Notes

- These Modelfiles set a 128k context (`num_ctx 131072`). If your machine is constrained, reduce this (e.g., 32768 or 65536).
- The system prompt instructs models to return either a message or a single tool call per turn with allowed tools: `shell` and `apply_patch`. This pairs with Codex’s Ollama JSON bridge for tool execution.
- Keep profile names the same (e.g., `cogito-3b`, `qwen2-5-coder-7b`) and either:
  - Override the model at runtime with `-c model="cogito-3b-codex"` (or `qwen2.5-coder-7b-codex`), or
  - Update `~/.codex/config.toml` profile `model` fields to point to the new slugs.

Automatic Modelfile generator

You can synthesize a Codex‑tuned Modelfile for any existing Ollama model using the helper binary built with the workspace:

- Build: `cargo build --release -p codex-ollama-tools`
- Usage:
  - `codex-ollama-modelfile --model qwen2.5-coder:7b --out modelfiles/myqwen/Modelfile`
  - `codex-ollama-modelfile --model deepseek-coder-v2:16b --out modelfiles/deepseekv2codex/Modelfile`
  - Then create the tuned model: `ollama create -f modelfiles/deepseekv2codex/Modelfile deepseek-coder-v2-codex`

This tool runs `ollama show <model> --modelfile`, preserves the original base settings, and appends Codex’s JSON‑tooling system prompt and conservative parameters (low temperature, top_p, fixed seed, large context window). It produces a Modelfile that is fully compatible with Codex’s tooling bridge.
