//! 便携 MCP 的严格声明边界；工作目录和凭据的实际应用由运行时负责。

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};
use std::collections::HashSet;
use url::{Host, Url};

use super::mcp_legacy::{PluginMcpDeclaration, PluginMcpParseOutcome, PluginMcpServerError};
use super::path_audit::validate_component_path;

pub(crate) const SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json";
const CLIENT_HEADERS: &[&str] = &[
    "accept",
    "authorization",
    "connection",
    "content-encoding",
    "content-length",
    "content-type",
    "host",
    "last-event-id",
    "mcp-protocol-version",
    "mcp-session-id",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "user-agent",
];

pub(crate) fn parse_portable_mcp(value: Value) -> Result<PluginMcpParseOutcome> {
    let object = value.as_object().context("portable MCP document must be an object")?;
    fields(object, &["$schema", "mcpServers"])?;
    if object.get("$schema").and_then(Value::as_str) != Some(SCHEMA) {
        bail!("unsupported portable MCP $schema; expected Agent Plugins 1.0.0 mcp.schema.json");
    }
    let servers = object
        .get("mcpServers")
        .and_then(Value::as_object)
        .context("portable MCP mcpServers must be an object")?;
    let mut outcome = PluginMcpParseOutcome::default();
    for (name, value) in servers {
        match normalize_server(value) {
            Ok(config) => {
                outcome.servers.push(PluginMcpDeclaration::Portable { name: name.clone(), config })
            }
            Err(error) => outcome
                .errors
                .push(PluginMcpServerError { name: name.clone(), message: error.to_string() }),
        }
    }
    Ok(outcome)
}

fn normalize_server(value: &Value) -> Result<Map<String, Value>> {
    let object = value.as_object().context("portable MCP server must be an object")?;
    match object.get("type").and_then(Value::as_str) {
        Some("stdio") => normalize_stdio(object),
        Some("streamable-http") => normalize_http(object),
        Some("sse") => {
            fields(object, &["type", "url", "headers"])?;
            string(object, "url")?;
            if let Some(headers) = object.get("headers") {
                string_map(headers, "headers")?;
            }
            bail!("portable MCP type=sse is not supported");
        }
        _ => bail!("portable MCP type must be stdio or streamable-http; unknown protocol rejected"),
    }
}

fn normalize_stdio(object: &Map<String, Value>) -> Result<Map<String, Value>> {
    fields(object, &["type", "command", "args", "env", "cwd"])?;
    let command = string(object, "command")?;
    if command.trim().is_empty() {
        bail!("portable MCP command must be nonempty");
    }
    if command.starts_with("./") {
        validate_component_path("command", command)
            .map_err(|_| anyhow::anyhow!("portable MCP command must be a contained ./ path"))?;
    } else if command.contains(['/', '\\'])
        || matches!(command.as_bytes(), [drive, b':', ..] if drive.is_ascii_alphabetic())
    {
        bail!("portable MCP command must be a bare executable or contained ./ path");
    }
    let args = object.get("args").cloned().unwrap_or_else(|| json!([]));
    if !args.as_array().is_some_and(|args| args.iter().all(Value::is_string)) {
        bail!("portable MCP args must be a string array");
    }
    let env = object.get("env").cloned().unwrap_or_else(|| json!({}));
    let mut normalized_env = Map::new();
    for (name, value) in string_map(&env, "env")? {
        if reserved_env(name) {
            bail!("portable MCP env cannot override reserved PLUGIN_ROOT/PLUGIN_DATA");
        }
        let name = if cfg!(windows) { name.to_ascii_uppercase() } else { name.clone() };
        if normalized_env.insert(name, value.clone()).is_some() {
            bail!("portable MCP env contains case-insensitive duplicate names");
        }
    }
    let cwd = if object.contains_key("cwd") { string(object, "cwd")? } else { "${PLUGIN_ROOT}" };
    let valid_cwd = if cwd == "./" {
        true
    } else if cwd.starts_with("./") {
        validate_component_path("cwd", cwd).is_ok()
    } else {
        ["${PLUGIN_ROOT}", "${PLUGIN_DATA}"].iter().any(|root| {
            cwd == *root
                || cwd.strip_prefix(&format!("{root}/")).is_some_and(|suffix| {
                    suffix.is_empty()
                        || validate_component_path("cwd", &format!("./{suffix}")).is_ok()
                })
        })
    };
    if !valid_cwd {
        bail!(
            "portable MCP cwd must be a contained ./, ${{PLUGIN_ROOT}}, or ${{PLUGIN_DATA}} path"
        );
    }
    Ok(Map::from_iter([
        ("command".into(), json!(command)),
        ("args".into(), args),
        ("env".into(), Value::Object(normalized_env)),
        ("cwd".into(), json!(cwd)),
    ]))
}

fn normalize_http(object: &Map<String, Value>) -> Result<Map<String, Value>> {
    fields(object, &["type", "url", "headers", "extensions"])?;
    let raw = string(object, "url")?;
    let url = Url::parse(raw)
        .map_err(|_| anyhow::anyhow!("portable MCP url must be absolute HTTP(S)"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        bail!("portable MCP url must be HTTP(S) without user information or fragment");
    }
    let loopback = match url.host() {
        Some(Host::Domain(host)) => host == "localhost",
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        None => false,
    };
    if url.scheme() == "http" && !loopback {
        bail!("portable MCP url must use HTTPS for non-loopback hosts");
    }
    let mut config = Map::from_iter([("url".into(), json!(raw))]);
    if let Some(headers) = object.get("headers") {
        let mut names = HashSet::new();
        let mut retained = Map::new();
        for (name, value) in string_map(headers, "headers")? {
            if name.is_empty()
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
            {
                bail!("portable MCP headers contain an invalid header name");
            }
            if !names.insert(name.to_ascii_lowercase()) {
                bail!("portable MCP headers contain case-insensitive duplicate names");
            }
            if value
                .as_str()
                .expect("validated string")
                .bytes()
                .any(|b| b < 32 && b != b'\t' || b == 127)
            {
                bail!("portable MCP headers contain an invalid header value");
            }
            if !CLIENT_HEADERS.iter().any(|owned| name.eq_ignore_ascii_case(owned)) {
                retained.insert(name.clone(), value.clone());
            }
        }
        if !retained.is_empty() {
            config.insert("http_headers".into(), Value::Object(retained));
        }
    }
    if let Some(extensions) = object.get("extensions") {
        config.insert("extensions".into(), extensions.clone());
        super::mcp_public_auth::policy(&config)?;
    }
    Ok(config)
}

fn fields(object: &Map<String, Value>, allowed: &[&str]) -> Result<()> {
    if let Some(field) = object.keys().find(|field| !allowed.contains(&field.as_str())) {
        bail!("unknown portable MCP field '{field}'");
    }
    Ok(())
}

fn string<'a>(object: &'a Map<String, Value>, field: &str) -> Result<&'a str> {
    object
        .get(field)
        .and_then(Value::as_str)
        .with_context(|| format!("portable MCP {field} must be a string"))
}

fn string_map<'a>(value: &'a Value, field: &str) -> Result<&'a Map<String, Value>> {
    let map =
        value.as_object().with_context(|| format!("portable MCP {field} must be a string map"))?;
    if !map.values().all(Value::is_string) {
        bail!("portable MCP {field} must be a string map");
    }
    Ok(map)
}

pub(super) fn env_names_match(left: &str, right: &str) -> bool {
    if cfg!(windows) { left.eq_ignore_ascii_case(right) } else { left == right }
}

pub(super) fn reserved_env(name: &str) -> bool {
    ["PLUGIN_ROOT", "PLUGIN_DATA"].iter().any(|reserved| env_names_match(name, reserved))
}
