//! 插件 MCP 的包外数据目录及进程上下文。ACP 扩展只传递宿主生成的路径。
use super::mcp_legacy::{PluginMcpDeclaration, PluginMcpParseOutcome, PluginMcpServerError};
use crate::plugins::{DiscoveredPlugin, PluginDiscoveryOptions};
use agent_client_protocol::{McpServer, McpServerStdio};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    path::{Component, Path, PathBuf},
};

const CONTEXT_KEY: &str = "shiju/plugin-process-context";
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProcessContext {
    root: PathBuf,
    data: PathBuf,
    cwd: PathBuf,
    package_command: bool,
}

/// 在配置归一化后、严格运行准入前绑定已发现的插件来源。
pub(crate) fn resolve_servers(
    mut outcome: PluginMcpParseOutcome,
    plugin: &DiscoveredPlugin,
    options: &PluginDiscoveryOptions,
    source: &str,
    original_names: &HashMap<String, String>,
) -> Result<Vec<McpServer>> {
    let mut http_contexts = super::mcp_auth::prepare(&mut outcome, plugin, original_names)?;
    let mut connection_settings =
        super::mcp_connection_settings::prepare(&mut outcome, &plugin.manifest.root)?;
    let mut contexts = HashMap::new();
    for declaration in &mut outcome.servers {
        let (name, config, portable) = match declaration {
            PluginMcpDeclaration::Legacy { name, config } => (name, config, false),
            PluginMcpDeclaration::Portable { name, config } => (name, config, true),
            PluginMcpDeclaration::Acp(McpServer::Stdio(server)) => {
                if server.meta.as_ref().is_some_and(|meta| meta.contains_key(CONTEXT_KEY)) {
                    outcome.errors.push(PluginMcpServerError {
                        name: server.name.clone(),
                        message: "plugin cannot supply host process context".into(),
                    });
                }
                continue;
            }
            _ => continue,
        };
        if !config.contains_key("command") || config.get("enabled") == Some(&Value::Bool(false)) {
            continue;
        }
        match resolve_stdio(config, portable, plugin, options) {
            Ok(Some(context)) => {
                contexts.insert(name.clone(), context);
            }
            Ok(None) => {}
            Err(error) => outcome
                .errors
                .push(PluginMcpServerError { name: name.clone(), message: error.to_string() }),
        }
    }
    let mut servers = outcome.into_runtime_servers(source)?;
    for server in &mut servers {
        if let Some(settings) =
            connection_settings.remove(crate::tools::mcp_common::server_name(server))
        {
            super::mcp_connection_settings::bind(server, settings)?;
        }
        if let McpServer::Http(server) = server
            && let Some(context) = http_contexts.remove(&server.name)
        {
            super::mcp_auth::bind(server, context)?;
        }
        if let McpServer::Stdio(server) = server
            && let Some(context) = contexts.remove(&server.name)
        {
            server
                .meta
                .get_or_insert_default()
                .insert(CONTEXT_KEY.into(), serde_json::to_value(context)?);
        }
    }
    Ok(servers)
}

fn resolve_stdio(
    config: &mut Map<String, Value>,
    portable: bool,
    plugin: &DiscoveredPlugin,
    options: &PluginDiscoveryOptions,
) -> Result<Option<ProcessContext>> {
    if config.get("environment_id").is_some_and(|value| !value.is_null()) {
        bail!("remote plugin MCP execution is not supported by local runtime");
    }
    let mut env = config.get("env").and_then(Value::as_object).cloned().unwrap_or_default();
    for name in env.keys() {
        validate_env_name(name)?;
        if reserved(name) {
            bail!("plugin MCP env cannot override reserved PLUGIN_ROOT/PLUGIN_DATA");
        }
    }
    if let Some(references) = config.remove("env_vars").filter(|value| !value.is_null()) {
        for reference in references.as_array().context("plugin MCP env_vars must be an array")? {
            let name = match reference {
                Value::String(name) => name.as_str(),
                Value::Object(reference)
                    if reference.get("source").and_then(Value::as_str) == Some("local")
                        && reference.len() == 2 =>
                {
                    reference
                        .get("name")
                        .and_then(Value::as_str)
                        .context("plugin MCP env_vars requires a name")?
                }
                _ => bail!("remote or unsupported plugin MCP environment source"),
            };
            validate_env_name(name)?;
            if reserved(name) {
                bail!("plugin MCP env_vars cannot override reserved PLUGIN_ROOT/PLUGIN_DATA");
            }
            let name = if cfg!(windows) { name.to_ascii_uppercase() } else { name.to_owned() };
            if !env.contains_key(&name) {
                let value = std::env::var(&name).map_err(|_| {
                    anyhow::anyhow!("required plugin MCP environment variable is unavailable")
                })?;
                env.insert(name.into(), Value::String(value));
            }
        }
    }
    let cwd = config
        .remove("cwd")
        .filter(|value| !value.is_null())
        .map(|value| value.as_str().map(str::to_owned).context("plugin MCP cwd must be a string"))
        .transpose()?;
    if !portable && cwd.is_none() {
        config.insert("env".into(), Value::Object(env));
        return Ok(None);
    }
    let root = fs::canonicalize(&plugin.manifest.root).context("failed to resolve plugin root")?;
    let data_root = options.config_dir.join("plugin-data");
    let data_root = resolve_prefix(&absolute(&data_root)?)?;
    if data_root.starts_with(&root) || root.starts_with(&data_root) {
        bail!("plugin data directory must be separate from package directory");
    }
    let mut hash = Sha256::new();
    hash.update(fs::canonicalize(&plugin.source_dir)?.as_os_str().as_encoded_bytes());
    hash.update([0]);
    hash.update(plugin.identity.as_key());
    let data = data_root.join(hex::encode(hash.finalize()));
    // 先审计已有前缀，再创建宿主状态目录；不在包内创建文件或 cwd。
    if resolve_prefix(&data)? != data {
        bail!("plugin data directory escapes host data root");
    }
    fs::create_dir_all(&data).context("failed to create plugin data directory")?;
    let canonical_data = fs::canonicalize(&data)?;
    if canonical_data != data {
        bail!("plugin data directory escapes host data root");
    }
    let raw = cwd.as_deref().unwrap_or("${PLUGIN_ROOT}");
    let (cwd, allowed_root) = if portable {
        let expanded = expand(raw, &root, &data)?;
        let allowed = if raw.starts_with("${PLUGIN_DATA}") { &data } else { &root };
        (
            if Path::new(&expanded).is_absolute() {
                PathBuf::from(expanded)
            } else {
                root.join(expanded)
            },
            allowed,
        )
    } else {
        if raw.contains("://") {
            bail!("remote plugin MCP cwd is not supported by local runtime");
        }
        (if Path::new(raw).is_absolute() { PathBuf::from(raw) } else { root.join(raw) }, &root)
    };
    let cwd = contained(&cwd, allowed_root)?;
    let command = config
        .get("command")
        .and_then(Value::as_str)
        .context("plugin MCP command must be a string")?;
    let package_command = portable && command.starts_with("./");
    if package_command {
        config.insert(
            "command".into(),
            Value::String(path_string(&contained(&root.join(command), &root)?)?),
        );
    }
    if portable {
        if let Some(args) = config.get_mut("args").and_then(Value::as_array_mut) {
            for arg in args {
                *arg = Value::String(expand(
                    arg.as_str().context("invalid plugin MCP argument")?,
                    &root,
                    &data,
                )?);
            }
        }
        for value in env.values_mut() {
            *value = Value::String(expand(
                value.as_str().context("invalid plugin MCP environment")?,
                &root,
                &data,
            )?);
        }
        env.insert("PLUGIN_ROOT".into(), Value::String(path_string(&root)?));
        env.insert("PLUGIN_DATA".into(), Value::String(path_string(&data)?));
    }
    config.insert("env".into(), Value::Object(env));
    Ok(Some(ProcessContext { root, data, cwd, package_command }))
}
fn reserved(name: &str) -> bool {
    ["PLUGIN_ROOT", "PLUGIN_DATA"]
        .iter()
        .any(|key| if cfg!(windows) { name.eq_ignore_ascii_case(key) } else { name == *key })
}
fn validate_env_name(name: &str) -> Result<()> {
    if name.is_empty() || name.contains(['=', '\0']) {
        bail!("invalid plugin MCP environment variable name");
    }
    Ok(())
}
fn path_string(path: &Path) -> Result<String> {
    path.to_str().map(str::to_owned).context("plugin MCP path must be UTF-8")
}
fn expand(value: &str, root: &Path, data: &Path) -> Result<String> {
    let root = path_string(root)?;
    let data = path_string(data)?;
    let mut output = String::new();
    let mut rest = value;
    while let Some(index) = rest.find("${PLUGIN_") {
        output.push_str(&rest[..index]);
        rest = &rest[index..];
        if let Some(after) = rest.strip_prefix("${PLUGIN_ROOT}") {
            output.push_str(&root);
            rest = after;
        } else if let Some(after) = rest.strip_prefix("${PLUGIN_DATA}") {
            output.push_str(&data);
            rest = after;
        } else {
            output.push_str("${PLUGIN_");
            rest = &rest[9..];
        }
    }
    output.push_str(rest);
    Ok(output)
}
fn absolute(path: &Path) -> Result<PathBuf> {
    Ok(if path.is_absolute() { path.to_owned() } else { std::env::current_dir()?.join(path) })
}
fn resolve_prefix(path: &Path) -> Result<PathBuf> {
    let mut existing = path.to_owned();
    let mut missing = Vec::new();
    loop {
        match fs::canonicalize(&existing) {
            Ok(mut path) => {
                for component in missing.iter().rev() {
                    path.push(component);
                }
                let mut normalized = PathBuf::new();
                for component in path.components() {
                    match component {
                        Component::ParentDir => {
                            normalized.pop();
                        }
                        Component::CurDir => {}
                        _ => normalized.push(component.as_os_str()),
                    }
                }
                return Ok(normalized);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if fs::symlink_metadata(&existing).is_ok_and(|meta| meta.file_type().is_symlink()) {
                    bail!("plugin MCP path contains a broken symlink");
                }
                let component = existing
                    .components()
                    .next_back()
                    .context("plugin MCP path has no existing root")?;
                if matches!(component, Component::RootDir | Component::Prefix(_)) {
                    bail!("plugin MCP path has no existing root");
                }
                missing.push(component.as_os_str().to_owned());
                existing.pop();
            }
            Err(_) => bail!("failed to resolve plugin MCP path"),
        }
    }
}
fn contained(path: &Path, root: &Path) -> Result<PathBuf> {
    let resolved = resolve_prefix(path)?;
    if !resolved.starts_with(root) {
        bail!("plugin MCP path is outside plugin roots");
    }
    Ok(resolved)
}
/// 再次检查动态路径，防止加载配置后替换 cwd/可执行文件的 symlink。
pub(crate) fn runtime_cwd(server: &McpServerStdio) -> Result<Option<PathBuf>> {
    let Some(value) = server.meta.as_ref().and_then(|meta| meta.get(CONTEXT_KEY)) else {
        return Ok(None);
    };
    let context: ProcessContext =
        serde_json::from_value(value.clone()).context("invalid plugin MCP process context")?;
    for root in [&context.root, &context.data] {
        if !root.is_absolute() || fs::canonicalize(root)? != *root {
            bail!("plugin MCP root changed after discovery");
        }
    }
    if context.data.starts_with(&context.root) || context.root.starts_with(&context.data) {
        bail!("plugin MCP data overlaps package directory");
    }
    let allowed =
        if context.cwd.starts_with(&context.root) { &context.root } else { &context.data };
    let cwd = contained(&context.cwd, allowed)?;
    if context.package_command {
        contained(&server.command, &context.root)?;
    }
    Ok(Some(cwd))
}
