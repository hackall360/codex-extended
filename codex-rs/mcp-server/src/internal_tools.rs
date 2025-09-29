use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use codex_apply_patch::apply_patch;
use codex_core::config::Config;
use codex_core::exec::ExecParams;
use codex_core::exec::ExecToolCallOutput;
use codex_core::exec::SandboxType;
use codex_core::exec::process_exec_tool_call;
use codex_core::exec_env::create_env;
use codex_core::get_platform_sandbox;
use codex_core::plan_tool::UpdatePlanArgs;
use codex_protocol::models::ShellToolCallParams;
use mcp_types::CallToolResult;
use mcp_types::ContentBlock;
use mcp_types::TextContent;
use mcp_types::Tool;
use mcp_types::ToolInputSchema;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::json;
use tokio::sync::Mutex;

pub struct InternalToolRegistry {
    config: Arc<Config>,
    plan_state: Mutex<Option<UpdatePlanArgs>>,
}

impl InternalToolRegistry {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            plan_state: Mutex::new(None),
        }
    }

    pub fn list_tools(&self) -> Vec<Tool> {
        vec![
            shell_tool(),
            apply_patch_tool(),
            update_plan_tool(),
            view_image_tool(),
        ]
    }

    pub async fn try_call_tool(
        &self,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Option<CallToolResult> {
        match name {
            "shell" | "container.exec" => Some(self.call_shell(arguments).await),
            "apply_patch" => Some(self.call_apply_patch(arguments).await),
            "update_plan" => Some(self.call_update_plan(arguments).await),
            "view_image" => Some(self.call_view_image(arguments).await),
            _ => None,
        }
    }

    async fn call_shell(&self, arguments: Option<serde_json::Value>) -> CallToolResult {
        let params: ShellToolCallParams = match parse_arguments(arguments) {
            Ok(value) => value,
            Err(err) => return error_result(err),
        };
        if params.command.is_empty() {
            return error_result("Shell tool requires a command to execute");
        }
        let cwd = resolve_path(&self.config.cwd, params.workdir.as_deref());
        let env = create_env(&self.config.shell_environment_policy);
        let exec_params = ExecParams {
            command: params.command,
            cwd,
            timeout_ms: params.timeout_ms,
            env,
            with_escalated_permissions: params.with_escalated_permissions,
            justification: params.justification,
        };

        let sandbox_type = match get_platform_sandbox() {
            Some(SandboxType::LinuxSeccomp) if self.config.codex_linux_sandbox_exe.is_some() => {
                SandboxType::LinuxSeccomp
            }
            Some(SandboxType::MacosSeatbelt) => SandboxType::MacosSeatbelt,
            _ => SandboxType::None,
        };

        match process_exec_tool_call(
            exec_params,
            sandbox_type,
            &self.config.sandbox_policy,
            &self.config.cwd,
            &self.config.codex_linux_sandbox_exe,
            None,
        )
        .await
        {
            Ok(output) => success_result(format_exec_output(&output)),
            Err(err) => error_result(format!("Failed to execute command: {err}")),
        }
    }

    async fn call_apply_patch(&self, arguments: Option<serde_json::Value>) -> CallToolResult {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ApplyPatchArgs {
            Structured { input: String },
            Legacy { patch: String },
        }

        let value: ApplyPatchArgs = match parse_arguments(arguments) {
            Ok(v) => v,
            Err(err) => return error_result(err),
        };
        let patch = match value {
            ApplyPatchArgs::Structured { input } => input,
            ApplyPatchArgs::Legacy { patch } => patch,
        };

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        match apply_patch(&patch, &mut stdout, &mut stderr) {
            Ok(()) => {
                let mut output = String::new();
                if !stdout.is_empty() {
                    output.push_str(&String::from_utf8_lossy(&stdout));
                }
                if !stderr.is_empty() {
                    if !output.is_empty() {
                        output.push('\n');
                    }
                    output.push_str(&String::from_utf8_lossy(&stderr));
                }
                if output.trim().is_empty() {
                    output = "Patch applied".to_string();
                }
                success_result(output)
            }
            Err(err) => error_result(format!("Failed to apply patch: {err}")),
        }
    }

    async fn call_update_plan(&self, arguments: Option<serde_json::Value>) -> CallToolResult {
        let args: UpdatePlanArgs = match parse_arguments(arguments) {
            Ok(value) => value,
            Err(err) => return error_result(err),
        };
        *self.plan_state.lock().await = Some(args.clone());
        match serde_json::to_string(&args) {
            Ok(serialized) => success_result(format!("Plan updated: {serialized}")),
            Err(_) => success_result("Plan updated".to_string()),
        }
    }

    async fn call_view_image(&self, arguments: Option<serde_json::Value>) -> CallToolResult {
        #[derive(Deserialize)]
        struct ViewImageArgs {
            path: String,
        }
        let args: ViewImageArgs = match parse_arguments(arguments) {
            Ok(value) => value,
            Err(err) => return error_result(err),
        };
        let resolved = resolve_path(&self.config.cwd, Some(args.path.as_str()));
        match std::fs::metadata(&resolved) {
            Ok(meta) if meta.is_file() => {
                success_result(format!("Image available at {}", resolved.display()))
            }
            Ok(_) => error_result(format!("Path {} is not a file", resolved.display())),
            Err(err) => error_result(format!("Failed to access {}: {err}", resolved.display())),
        }
    }
}

fn parse_arguments<T>(arguments: Option<serde_json::Value>) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let value = arguments.ok_or_else(|| "Missing arguments".to_string())?;
    serde_json::from_value(value).map_err(|err| format!("Failed to parse arguments: {err}"))
}

fn resolve_path(base: &Path, path: Option<&str>) -> PathBuf {
    match path {
        Some(p) => {
            let candidate = Path::new(p);
            if candidate.is_absolute() {
                candidate.to_path_buf()
            } else {
                base.join(candidate)
            }
        }
        None => base.to_path_buf(),
    }
}

fn shell_tool() -> Tool {
    let properties = json!({
        "command": {
            "type": "array",
            "items": { "type": "string" },
            "description": "The command to execute"
        },
        "workdir": {
            "type": "string",
            "description": "Working directory to run the command in"
        },
        "timeout_ms": {
            "type": "number",
            "description": "Timeout for the command in milliseconds"
        }
    });
    Tool {
        name: "shell".to_string(),
        title: Some("Shell".to_string()),
        description: Some("Run a shell command and return its output.".to_string()),
        input_schema: ToolInputSchema {
            r#type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["command".to_string()]),
        },
        output_schema: None,
        annotations: None,
    }
}

fn apply_patch_tool() -> Tool {
    let properties = json!({
        "input": {
            "type": "string",
            "description": "Unified diff describing the file changes"
        }
    });
    Tool {
        name: "apply_patch".to_string(),
        title: Some("Apply Patch".to_string()),
        description: Some("Apply a unified diff to the local workspace.".to_string()),
        input_schema: ToolInputSchema {
            r#type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["input".to_string()]),
        },
        output_schema: None,
        annotations: None,
    }
}

fn update_plan_tool() -> Tool {
    let plan_item_schema = json!({
        "type": "object",
        "properties": {
            "step": { "type": "string" },
            "status": {
                "type": "string",
                "description": "One of: pending, in_progress, completed"
            }
        },
        "required": ["step", "status"],
        "additionalProperties": false
    });
    let properties = json!({
        "plan": {
            "type": "array",
            "description": "List of plan items",
            "items": plan_item_schema
        },
        "explanation": {
            "type": "string",
            "description": "Optional explanation of the plan"
        }
    });
    Tool {
        name: "update_plan".to_string(),
        title: Some("Update Plan".to_string()),
        description: Some(
            "Record the current task plan with step statuses for external observers.".to_string(),
        ),
        input_schema: ToolInputSchema {
            r#type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["plan".to_string()]),
        },
        output_schema: None,
        annotations: None,
    }
}

fn view_image_tool() -> Tool {
    let properties = json!({
        "path": {
            "type": "string",
            "description": "Filesystem path to an image file"
        }
    });
    Tool {
        name: "view_image".to_string(),
        title: Some("View Image".to_string()),
        description: Some(
            "Attach a local image path so other agents can reference the file.".to_string(),
        ),
        input_schema: ToolInputSchema {
            r#type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["path".to_string()]),
        },
        output_schema: None,
        annotations: None,
    }
}

fn success_result(message: impl Into<String>) -> CallToolResult {
    CallToolResult {
        content: vec![ContentBlock::TextContent(TextContent {
            r#type: "text".to_string(),
            text: message.into(),
            annotations: None,
        })],
        is_error: Some(false),
        structured_content: None,
    }
}

fn error_result(message: impl Into<String>) -> CallToolResult {
    CallToolResult {
        content: vec![ContentBlock::TextContent(TextContent {
            r#type: "text".to_string(),
            text: message.into(),
            annotations: None,
        })],
        is_error: Some(true),
        structured_content: None,
    }
}

fn format_exec_output(output: &ExecToolCallOutput) -> String {
    let mut sections: Vec<String> = Vec::new();
    sections.push(format!(
        "Exit code: {}\nDuration: {:.3} seconds",
        output.exit_code,
        output.duration.as_secs_f32()
    ));
    if output.timed_out {
        sections.push("Command timed out".to_string());
    }
    if !output.stdout.text.trim().is_empty() {
        sections.push(format!("Stdout:\n{}", output.stdout.text));
    }
    if let Some(lines) = output.stdout.truncated_after_lines {
        sections.push(format!("Stdout truncated after {lines} lines"));
    }
    if !output.stderr.text.trim().is_empty() {
        sections.push(format!("Stderr:\n{}", output.stderr.text));
    }
    if let Some(lines) = output.stderr.truncated_after_lines {
        sections.push(format!("Stderr truncated after {lines} lines"));
    }
    if !output.aggregated_output.text.trim().is_empty() {
        sections.push(format!(
            "Combined output:\n{}",
            output.aggregated_output.text
        ));
    }
    if let Some(lines) = output.aggregated_output.truncated_after_lines {
        sections.push(format!("Combined output truncated after {lines} lines"));
    }
    sections.join("\n\n")
}
