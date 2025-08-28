use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::config_types::McpServerConfig;

const MANIFEST_FILE: &str = "plugin.toml";

#[derive(Debug, Deserialize)]
struct PluginManifest {
    /// Optional explicit plugin name. Falls back to directory name when unset.
    name: Option<String>,
    /// MCP server configuration for this plugin. Additional plugin types can
    /// be added later; v1 focuses on MCP which Codex already supports.
    mcp: Option<PluginMcp>,
}

#[derive(Debug, Deserialize)]
struct PluginMcp {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: Option<HashMap<String, String>>,
}

/// Returns `true` if `name` is a valid MCP server name (matches
/// `^[a-zA-Z0-9_-]+$`). Keep in sync with `mcp_connection_manager.rs`.
fn is_valid_server_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Discover plugin manifests from `cwd/plugins` and `codex_home/plugins`.
/// Project-local plugins take precedence over global ones when names collide.
///
/// Returns the discovered MCP server map and a vector of human-readable
/// warnings encountered during discovery.
pub(crate) fn discover_mcp_plugins(
    cwd: &Path,
    codex_home: &Path,
) -> (HashMap<String, McpServerConfig>, Vec<String>) {
    let mut servers: HashMap<String, McpServerConfig> = HashMap::new();
    let mut warnings: Vec<String> = Vec::new();

    let global = codex_home.join("plugins");
    let project = cwd.join("plugins");

    // Load order: global first, then project overrides with same name.
    for root in [global, project] {
        if !root.exists() {
            continue;
        }
        let dir_entries = match fs::read_dir(&root) {
            Ok(it) => it,
            Err(e) => {
                warnings.push(format!(
                    "failed to read plugins directory {}: {e}",
                    root.display()
                ));
                continue;
            }
        };

        for entry_res in dir_entries {
            let entry = match entry_res {
                Ok(e) => e,
                Err(e) => {
                    warnings.push(format!("failed to read a plugin entry: {e}"));
                    continue;
                }
            };
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let manifest_path = path.join(MANIFEST_FILE);
            if !manifest_path.exists() {
                continue;
            }

            match parse_manifest(&manifest_path, &path) {
                Ok((name, cfg)) => {
                    // Last-wins (project overrides global) due to load order.
                    servers.insert(name, cfg);
                }
                Err(msg) => warnings.push(msg),
            }
        }
    }

    (servers, warnings)
}

fn parse_manifest(
    manifest_path: &Path,
    plugin_dir: &Path,
) -> Result<(String, McpServerConfig), String> {
    let s = fs::read_to_string(manifest_path).map_err(|e| {
        format!(
            "failed to read plugin manifest {}: {e}",
            manifest_path.display()
        )
    })?;

    let manifest: PluginManifest = toml::from_str(&s).map_err(|e| {
        format!(
            "failed to parse plugin manifest {}: {e}",
            manifest_path.display()
        )
    })?;

    let mcp = match manifest.mcp {
        Some(m) => m,
        None => {
            return Err(format!(
                "plugin manifest {} missing [mcp] section",
                manifest_path.display()
            ));
        }
    };

    let name = match manifest.name {
        Some(n) => n,
        None => match plugin_dir.file_name().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => {
                return Err(format!(
                    "could not derive plugin name from directory: {}",
                    plugin_dir.display()
                ));
            }
        },
    };
    if !is_valid_server_name(&name) {
        return Err(format!(
            "plugin name '{}' from {} is invalid (expected ^[a-zA-Z0-9_-]+$)",
            name,
            manifest_path.display()
        ));
    }

    let command = resolve_command(&mcp.command, plugin_dir);
    Ok((
        name,
        McpServerConfig {
            command,
            args: mcp.args,
            env: mcp.env,
        },
    ))
}

/// Resolve relative command paths against the plugin directory. Bare program
/// names (no separators) are left as-is to allow PATH lookup.
fn resolve_command(cmd: &str, plugin_dir: &Path) -> String {
    let has_sep = cmd.contains('/') || cmd.contains('\\');
    if has_sep {
        let p = PathBuf::from(cmd);
        if p.is_absolute() {
            cmd.to_string()
        } else {
            plugin_dir.join(p).to_string_lossy().to_string()
        }
    } else {
        cmd.to_string()
    }
}
