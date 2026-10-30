//! Codex 便携包只允许旧版独立 overlay 添加本地环境引用，不替换服务器或传输。

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;

use super::super::PluginMcpServers;
use super::super::audit::resolve_component_path;
use super::legacy_manifest::parse_legacy_manifest;
use super::mcp_legacy::{PluginMcpDeclaration, PluginMcpParseOutcome, parse_legacy_mcp};
use super::mcp_portable::{env_names_match, reserved_env};

#[derive(Deserialize, Serialize)]
#[serde(untagged, deny_unknown_fields)]
enum EnvReference {
    Name(String),
    Config {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<String>,
    },
}

impl EnvReference {
    fn name(&self) -> &str {
        match self {
            Self::Name(name) | Self::Config { name, .. } => name,
        }
    }
}

pub(crate) fn apply_env_overlay(root: &Path, outcome: &mut PluginMcpParseOutcome) -> Result<()> {
    if outcome.servers.is_empty() {
        return Ok(());
    }
    let path = root.join(".codex-plugin/plugin.json");
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).context("failed to inspect portable MCP overlay"),
        Ok(_) => {}
    }
    let path = resolve_component_path(root, "MCP overlay", "./.codex-plugin/plugin.json")?;
    let manifest =
        parse_legacy_manifest(root, &path.canonical_path, &fs::read(&path.canonical_path)?)?;
    let value = match manifest.mcp_servers {
        Some(PluginMcpServers::Inline(servers)) => Value::Object(servers),
        Some(PluginMcpServers::Path(path)) => {
            serde_json::from_slice(&fs::read(path.canonical_path)?)?
        }
        None => {
            if fs::symlink_metadata(root.join(".mcp.json"))
                .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
            {
                return Ok(());
            }
            let path = resolve_component_path(root, "MCP overlay", "./.mcp.json")?;
            serde_json::from_slice(&fs::read(path.canonical_path)?)?
        }
    };
    let overlay = parse_legacy_mcp(value, "portable MCP overlay")?;
    // Rejected overlay entries cannot supply environment references. Valid siblings remain usable.
    for declaration in overlay.servers {
        let PluginMcpDeclaration::Legacy { name, config } = declaration else { continue };
        if !config.contains_key("command") {
            continue;
        }
        let Some(PluginMcpDeclaration::Portable { config: target, .. }) = outcome.servers.iter_mut().find(|server| {
            matches!(server, PluginMcpDeclaration::Portable { name: target, config } if *target == name && config.contains_key("command"))
        }) else { continue };
        let Some(values) = config.get("env_vars") else { continue };
        let references: Vec<EnvReference> =
            serde_json::from_value(values.clone()).map_err(|_| {
                anyhow::anyhow!(
                    "portable MCP overlay env_vars must contain local variable references"
                )
            })?;
        let mut retained = Vec::new();
        for reference in references {
            if let EnvReference::Config { source: Some(source), .. } = &reference {
                match source.as_str() {
                    "remote" => continue,
                    "local" => {}
                    _ => bail!("portable MCP overlay env_vars has an unsupported source"),
                }
            }
            let name = reference.name();
            if reserved_env(name) {
                bail!(
                    "portable MCP overlay env_vars cannot override reserved PLUGIN_ROOT/PLUGIN_DATA"
                );
            }
            if let Some(env) = target.get_mut("env").and_then(Value::as_object_mut) {
                env.retain(|key, value| {
                    !env_names_match(key, name)
                        || value
                            .as_str()
                            .and_then(|value| value.strip_prefix("${"))
                            .and_then(|value| value.strip_suffix('}'))
                            .is_none_or(|value| !env_names_match(value, name))
                });
            }
            retained.push(serde_json::to_value(reference)?);
        }
        if !retained.is_empty() {
            target.insert("env_vars".into(), Value::Array(retained));
        }
    }
    Ok(())
}
