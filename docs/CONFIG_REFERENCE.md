Codex Configuration Reference

This document lists all supported configuration fields, environment variables, and CLI/tooling options across the Codex system, including the new multi‑agent orchestration, search, RAG, and DAG functionality.

Config sources are merged with this precedence: config.toml < CLI overrides (-c key=value) < strong overrides (programmatic) < environment variables.

Core config.toml fields (codex-rs/core)

- model: Model slug used by the primary manager agent.
- model_family: Inferred from model; controls defaults like apply_patch tool type and local shell support.
- model_context_window: Optional u64; context window tokens.
- model_max_output_tokens: Optional u64; max output tokens.
- model_provider_id: String key into model_providers map.
- model_provider: Provider definition for HTTP client (derived from provider map + overrides).
- approval_policy: Command approval policy; values: untrusted|on-failure|on-request|never.
- sandbox_policy: Tool sandbox policy; values: danger-full-access|read-only|workspace-write.
- shell_environment_policy: Policy controlling env var inheritance; see section below.
- hide_agent_reasoning: bool; suppress AgentReasoning events in UI.
- show_raw_agent_reasoning: bool; include AgentReasoningRawContent events.
- disable_response_storage: bool; disable server-side storage; send full history each turn.
- user_instructions: Optional string; appended to system prompt.
- base_instructions: Optional string; overrides the base system prompt fully.
- notify: Optional array of strings; command to invoke after each agent turn with JSON payload appended.
- cwd: Filesystem path for session; resolves relative tool paths.
- mcp_servers: Map<string, {command, args[], env{}}>; declares MCP servers.
- model_providers: Map<string, ModelProviderInfo>; provider connection settings.
- model_roles: Map<string, {model, provider?}>; reusable model/provider presets.
- experimental_resume: Optional path; internal resume data path.
- history: { persistence: save-all|none, max_bytes?: usize }.
- tui: { auto_compact_enabled?: bool, auto_compact_start_percent?: u8, auto_compact_reduction_step_percent?: u8, auto_compact_tolerance_percent?: u8 }.
- include_plan_tool: bool; expose update_plan tool.
- include_apply_patch_tool: bool; force include apply_patch even if family default would omit.
- tools_web_search_request: bool; expose legacy web_search request tool (not required for new web agent).
- responses_originator_header: String; custom originator header for Responses API.
- preferred_auth_method: chatgpt|api-key.
- use_experimental_streamable_shell_tool: bool; expose exec_command/write_stdin tools.
- include_view_image_tool: bool; allow view_image tool.
- edit_mode: request|block|trusted; edit approval model.
- command_timeout_ms: integer milliseconds or "none".

ShellEnvironmentPolicy (toml)

- inherit: bool; inherit current env.
- ignore_default_excludes: bool; disable default excludes.
- exclude: [String]; case insensitive patterns to remove variables (default excludes include secrets).
- set: { KEY=VALUE, ... } set or override specific variables.
- include_only: [String]; restrict to only these variables.
- experimental_use_profile: bool; use OS profile loading.

SandboxPolicy (toml)

- danger-full-access: No restrictions.
- read-only: Read-only filesystem.
- workspace-write: Read-only with writes allowed to cwd.
  - sandbox_workspace_write: { read_roots: [Path], writable_roots: [Path], network_access: bool, exclude_tmpdir_env_var: bool, exclude_slash_tmp: bool }

ModelProviderInfo (derived from config + providers)

- wire_api: responses|chat|custom.
- base_url?: String; OpenAI-compatible base URL (OSS servers / Azure/OpenAI proxies).
- api_version?: String; provider API version.
- headers?: Map<string,string>; additional HTTP headers.
- env_key?: String; environment variable name for API key.
- organization?: String; org/account id header.
- stream_max_retries?: u32; reconnect budget for SSE streams.

Agent/Orchestration tools (function-call APIs)

- invoke_coding_agent(task, model_role?) -> {success,summary,details?}
- invoke_file_search_agent(query, limit?, level?) -> {total,matches}
  - level: small|medium|codebase|extra|full (scope hint; agent maps to sensible defaults)
- invoke_web_search_agent(query, top_k?, mode?) -> {documents:[{title,url,text}], provider}
  - mode: normal|deep|research (increasing breadth/depth)
- invoke_rag_agent(question, top_k?, level?, include_web?, include_local?) -> {summary,contexts:[{provenance,text}]}
- invoke_dag_agent(nodes, edges) -> {results:{nodeId->{summary,details}}, log:[...]}
  - nodes: [{ id, kind: web_search|file_search|rag|coding|autogen, params: {...} }]
  - edges: [{ from, to }]
  - autogen params: { prompt, provider_id?, model?, base_url? }

Environment variables

- CODEX_SERVER_URL: Base URL for embeddings endpoints (e.g., http://localhost:8080). Enables embeddings-backed RAG.
- CODEX_WEBSEARCH_PROVIDER: duckduckgo (default) | google_cse.
- CODEX_GOOGLE_CSE_KEY: API key for Google Custom Search.
- CODEX_GOOGLE_CSE_CX: CSE engine id.
- CODEX_HOME: Override default state directory (~/.codex).

Server config (crates/codex-server)

- port: u16; HTTP listen port (via Config mapping if server uses the same shared config).
- backend_url: URL for the backing LLM/embedding provider (e.g. Ollama endpoint).
- data_path: path to AOF/log data for embedded Redis-like store.
- resp_port?: optional RESP port to expose embedded store.

CLI flags and overrides

- -c key=value: Apply dotted-path overrides to config (applies before typed overrides).
- Other per-command flags follow CLI help; agents and tools are driven via conversations and function calls.

Provider families

- OpenAI/Chat-compatible backends: use wire_api: chat, tool choice auto, parallel_tool_calls disabled by default for tool serialism.
- Responses API backends: use wire_api: responses; tool definitions expose strict schema when required.
- OSS providers: configure base_url; set env_key to pick up API key from env.

Server/Client split

- Server provides embeddings, vector index, and RAG endpoints. Can run local models or forward to APIs. Uses an embedded Redis-like store for vectors.
- Client (TUI/CLI/core) performs all local file/search/exec operations. Configure CODEX_SERVER_URL on the client to delegate embeddings to the server.
- Terminal executions and file edits occur exclusively on the client’s system by design.


### Orchestration Model Selection
- Use `model_roles` in config.toml to define named roles (model + optional provider).
- Pass `model_role` (coding agent) or `answer_model_role` (rag agent) to choose models per call or DAG node.
- For AutoGen DAG nodes, specify `provider_id` and `model` or `base_url` to point to local/OSS endpoints.
- Embeddings endpoint can be set via `CODEX_SERVER_URL`; configure codex-server to use your desired embedding model.

### Examples

Define model roles for orchestration:

```toml
[model_roles.coding]
model = "qwen2.5-coder"
provider = "oss_ollama"  # must exist in [model_providers]

[model_roles.rag_answer]
model = "gpt-4o-mini"
provider = "openai"
```

Use agents with explicit roles:

- Coding: `invoke_coding_agent({"task":"Refactor module","model_role":"coding"})`
- RAG: `invoke_rag_agent({"question":"Explain X","answer_model_role":"rag_answer","include_web":true})`

Embeddings endpoint (client side):

```shell
export CODEX_SERVER_URL=http://localhost:8080
```

codex-server (server side) embedding model selection depends on its `backend_url` and server configuration. Point it at your local models (e.g., Ollama) or API providers, and it will serve embeddings via `/v1/embeddings`.
