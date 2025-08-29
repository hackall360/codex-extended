# Codex Plugins

Codex supports drop-in plugins that extend the agent with additional tools -- without changing any built-in behavior. Plugins are simple MCP servers discovered from well-known folders and auto-registered at startup.

- Project plugins: `<project>/plugins/<plugin>/...`
- Global plugins: `~/.codex/plugins/<plugin>/...`

Each plugin lives in its own subfolder and must include a `plugin.toml` manifest. The process you launch can be written in any language as long as it speaks MCP (Model Context Protocol) over stdio.

Codex already understands MCP; this layer just makes it easy to add your own servers without editing `config.toml`.

## Discovery and Precedence

Load order and override behavior:

1. Global plugins (`~/.codex/plugins/*/plugin.toml`)
2. Project plugins (`<project>/plugins/*/plugin.toml`) -- override globals with the same name
3. Explicit `mcp_servers` from `config.toml` -- override discovered plugins

If a manifest fails to parse or a plugin name is invalid, Codex logs a warning and skips it (no crash).

## Naming Rules

- `name` must match `^[a-zA-Z0-9_-]+$`.
- The `name` becomes the MCP server name; each tool is qualified internally as `<server>__<tool>`.
- Tool names must also respect MCP's allowed characters.

## Manifest Schema (`plugin.toml`)

```toml
# Optional; defaults to the folder name if omitted.
name = "complex_math"

[mcp]
# Program to launch (bare name on PATH, absolute path, or relative to the plugin folder)
command = "python"
args = ["run.py"]

# Optional environment for the process
# env = { PYTHONUNBUFFERED = "1" }
```

## Implementing an MCP Server

Minimum methods to implement over JSON-RPC 2.0:

- `initialize`: return protocol and server info, plus `capabilities.tools`
- `tools/list`: return the tools your server provides with JSON Schema input
- `tools/call`: execute a named tool, return a `CallToolResult` with `content` (e.g., text blocks) and optional `structuredContent`

Codex handles the rest: starting your server, listing tools, exposing them to the model, and forwarding tool calls.

## Example Plugin: Complex Math Helper

This repository includes a working example at `plugins/complex_math/` that Codex will discover and load when run from the repo root.

- Tools provided:
  - `calculate(expr: string)`: safely evaluate math expressions (+, -, *, /, **, parentheses, `sin`, `cos`, `tan`, `log`, `exp`, `sqrt`, `abs`, `pow`, and constants `pi`, `e`, `tau`).
  - `quadratic_solve(a: number, b: number, c: number)`: solve `ax^2 + bx + c = 0` (real or complex roots).
  - `matrix_det(matrix: number[][])`: determinant of a 2x2 or 3x3 matrix.

Folder layout:

```
plugins/
  complex_math/
    plugin.toml
    requirements.txt
    run.py
    server.py
```

`plugin.toml`:

```toml
name = "complex_math"

[mcp]
command = "python"
args = ["run.py"]
```

`run.py` bootstraps a virtual environment using `requirements.txt` and then executes `server.py`, a small, dependency-free JSON-RPC MCP server. `server.py`:

- Responds to `initialize` with `tools` capability
- Implements `tools/list` with the schemas above
- Implements `tools/call` to perform the math, returning both a human-readable text block and machine-readable `structuredContent`

When Codex runs in this repository, it will automatically start the `complex_math` server and expose its tools to the model as function calls named `complex_math__calculate`, `complex_math__quadratic_solve`, and `complex_math__matrix_det`.

## Testing Your Plugin

1. Place your plugin under `<project>/plugins/<name>` with a `plugin.toml` manifest.
2. Ensure your command is runnable (on PATH or referenced relatively/absolutely).
3. Run Codex from the project root; check logs for any plugin startup issues.
4. Ask the model to use your tool by name; for example:
   - "Use `complex_math__calculate` to evaluate `sin(pi/4)^2`."

## Tips and Security Considerations

- Keep servers isolated; avoid launching untrusted binaries.
- Validate inputs rigorously and cap execution time within your server.
- For cross-platform use, prefer portable commands, or provide per-OS scripts.
- Returning both `content` (user-friendly) and `structuredContent` (machine-friendly) helps downstream UIs.

