//! 旧版 MCP 文档归一化。声明保留与运行准入分离，未实现的设置明确拒绝。

use agent_client_protocol::{EnvVariable, McpServer, McpServerHttp, McpServerStdio};
use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};
use std::path::PathBuf;

// 不派生 Debug：声明可能包含环境值或公开服务的鉴权配置。
pub(crate) enum PluginMcpDeclaration {
    Acp(McpServer),
    Legacy { name: String, config: Map<String, Value> },
    Portable { name: String, config: Map<String, Value> },
}

pub(crate) struct PluginMcpServerError {
    pub name: String,
    pub message: String,
}

#[derive(Default)]
pub(crate) struct PluginMcpParseOutcome {
    pub servers: Vec<PluginMcpDeclaration>,
    pub errors: Vec<PluginMcpServerError>,
}

impl PluginMcpParseOutcome {
    /// 保留宿主现有严格准入：诊断单项错误后，不能部分应用失败的配置。
    pub(crate) fn into_runtime_servers(self, source: &str) -> Result<Vec<McpServer>> {
        let mut servers = Vec::new();
        let mut errors = self.errors;
        for declaration in self.servers {
            match declaration {
                PluginMcpDeclaration::Acp(server) => servers.push(server),
                PluginMcpDeclaration::Legacy { name, config }
                | PluginMcpDeclaration::Portable { name, config } => {
                    if config.get("enabled") == Some(&Value::Bool(false)) {
                        continue;
                    }
                    match runtime_server(&name, &config) {
                        Ok(server) => servers.push(server),
                        Err(error) => {
                            errors.push(PluginMcpServerError { name, message: error.to_string() })
                        }
                    }
                }
            }
        }
        if !errors.is_empty() {
            let messages = errors
                .iter()
                .map(|error| format!("server '{}': {}", error.name, error.message))
                .collect::<Vec<_>>()
                .join("; ");
            bail!("invalid plugin MCP declarations in {source}: {messages}");
        }
        Ok(servers)
    }
}

pub(crate) fn parse_legacy_mcp(value: Value, source: &str) -> Result<PluginMcpParseOutcome> {
    let declarations = match value {
        Value::Object(mut object) if object.contains_key("mcpServers") => {
            object.remove("mcpServers").expect("key checked")
        }
        other => other,
    };
    let mut outcome = PluginMcpParseOutcome::default();
    match declarations {
        Value::Object(servers) => {
            for (name, value) in servers {
                match normalize_server(&name, value) {
                    Ok(config) => {
                        outcome.servers.push(PluginMcpDeclaration::Legacy { name, config })
                    }
                    Err(error) => outcome
                        .errors
                        .push(PluginMcpServerError { name, message: error.to_string() }),
                }
            }
        }
        Value::Array(servers) => {
            for (index, value) in servers.into_iter().enumerate() {
                let name = value
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("#{index}"));
                match vw_acp::parse_mcp_servers(&Value::Array(vec![value]), source) {
                    Ok(servers) => {
                        outcome.servers.extend(servers.into_iter().map(PluginMcpDeclaration::Acp))
                    }
                    Err(error) => outcome
                        .errors
                        .push(PluginMcpServerError { name, message: error.to_string() }),
                }
            }
        }
        _ => bail!(
            "invalid plugin MCP config in {source}: expected array or object (mcpServers or direct server map)"
        ),
    }
    Ok(outcome)
}

fn normalize_server(name: &str, value: Value) -> Result<Map<String, Value>> {
    if name.trim().is_empty() {
        bail!("server name must not be empty");
    }
    let mut config = value.as_object().cloned().context("expected server object")?;
    if let Some(kind) = config.remove("type") {
        let kind = kind.as_str().context("type must be a string")?;
        if !matches!(kind, "http" | "streamable_http" | "streamable-http" | "stdio") {
            // 参考实现不以 type 决定传输；不要把未知标签或 SSE 警告当成能力。
            tracing::warn!(
                server = name,
                "legacy MCP transport label ignored; transport follows command/url"
            );
        }
    }
    if let Some(oauth) = config.get_mut("oauth").filter(|value| !value.is_null()) {
        let oauth = oauth.as_object_mut().context("oauth must be an object or null")?;
        for (old, new) in [
            ("callbackUrl", "callback_url"),
            ("callbackPort", "callback_port"),
            ("clientId", "client_id"),
            ("clientSecret", "client_secret"),
        ] {
            if let Some(value) = oauth.remove(old) {
                oauth.entry(new).or_insert(value);
            }
        }
    }
    if config.get("oauth").and_then(Value::as_object).is_some_and(Map::is_empty) {
        config.remove("oauth");
    }
    if let Some(value) = config.get("name").filter(|value| !value.is_null()) {
        value.as_str().context("legacy display name must be a string")?;
    }
    config.remove("name");
    if let Some(value) = config.get("enabled").filter(|value| !value.is_null()) {
        value.as_bool().context("enabled must be a boolean")?;
    }
    let has_command = config.get("command").is_some_and(|value| !value.is_null());
    let has_url = config.get("url").is_some_and(|value| !value.is_null());
    match (has_command, has_url) {
        (true, true) => bail!("command and url cannot be combined"),
        (true, false) => {
            required_string(&config, "command")?;
            if let Some(args) = config.get("args").filter(|value| !value.is_null()) {
                let args = args.as_array().context("args must be a string array")?;
                if !args.iter().all(Value::is_string) {
                    bail!("args must be a string array");
                }
            }
            if let Some(env) = config.get("env").filter(|value| !value.is_null()) {
                string_map(env, "env")?;
            }
        }
        (false, true) => {
            required_string(&config, "url")?;
            if ["args", "env"]
                .iter()
                .any(|field| config.get(*field).is_some_and(|value| !value.is_null()))
            {
                bail!("args and env are not supported for HTTP servers");
            }
        }
        (false, false) => bail!("expected command or url"),
    }
    Ok(config)
}

fn required_string<'a>(config: &'a Map<String, Value>, field: &str) -> Result<&'a str> {
    config
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .with_context(|| format!("{field} must be a non-empty string"))
}

fn string_map<'a>(value: &'a Value, field: &str) -> Result<&'a Map<String, Value>> {
    let object = value.as_object().with_context(|| format!("{field} must be a string map"))?;
    if !object.iter().all(|(name, value)| !name.is_empty() && value.is_string()) {
        bail!("{field} must contain non-empty names and string values");
    }
    Ok(object)
}

fn runtime_server(name: &str, config: &Map<String, Value>) -> Result<McpServer> {
    // 后续运行目录、鉴权和生命周期任务接入这些设置之前，不能静默丢弃它们。
    for (field, value) in config {
        if !value.is_null()
            && !matches!(field.as_str(), "command" | "args" | "env" | "url" | "enabled")
        {
            bail!("plugin MCP setting '{field}' is not supported by the runtime yet");
        }
    }
    if config.get("command").is_some_and(|value| !value.is_null()) {
        if config.get("url").is_some_and(|value| !value.is_null()) {
            bail!("command and url cannot be combined");
        }
        let command = required_string(config, "command")?;
        let args = config
            .get("args")
            .and_then(Value::as_array)
            .map(|args| {
                args.iter()
                    .map(|arg| arg.as_str().expect("validated argument").to_owned())
                    .collect()
            })
            .unwrap_or_default();
        let env = config
            .get("env")
            .filter(|value| !value.is_null())
            .map(|value| {
                string_map(value, "env").map(|env| {
                    env.iter()
                        .map(|(name, value)| {
                            EnvVariable::new(
                                name.clone(),
                                value.as_str().expect("validated environment value").to_owned(),
                            )
                        })
                        .collect()
                })
            })
            .transpose()?
            .unwrap_or_default();
        Ok(McpServer::Stdio(McpServerStdio::new(name, PathBuf::from(command)).args(args).env(env)))
    } else {
        // 静态 HTTP 头在公开鉴权任务中接入，避免在声明层伪造权限支持。
        Ok(McpServer::Http(McpServerHttp::new(name, required_string(config, "url")?)))
    }
}
