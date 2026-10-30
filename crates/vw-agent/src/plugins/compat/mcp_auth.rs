//! HTTP credentials are read at connection time, never copied into serializable ACP metadata.
use super::mcp_legacy::{PluginMcpDeclaration, PluginMcpParseOutcome, PluginMcpServerError};
use crate::plugins::{DiscoveredPlugin, load_plugin_manifest};
use agent_client_protocol::{McpServer, McpServerHttp};
use anyhow::{Context, Result, bail};
use reqwest_012::header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
};
use url::{Host, Url};

const CONTEXT_KEY: &str = "shiju/plugin-http-context";
const AUTH_FIELDS: &[&str] = &[
    "http_headers",
    "env_http_headers",
    "bearer_token_env_var",
    "oauth",
    "oauth_resource",
    "scopes",
    "auth",
    "extensions",
];

// Only paths, routing identity and digests travel through ACP. No header or OAuth secret values.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HttpContext {
    root: PathBuf,
    manifest: PathBuf,
    scope: String,
    name: String,
    url: String,
    digest: String,
}

pub(crate) fn prepare(
    outcome: &mut PluginMcpParseOutcome,
    plugin: &DiscoveredPlugin,
) -> Result<HashMap<String, HttpContext>> {
    let mut contexts = HashMap::new();
    for declaration in &mut outcome.servers {
        if forged_context(declaration) {
            outcome.errors.push(PluginMcpServerError {
                name: declaration_name(declaration).to_owned(),
                message: "plugin cannot supply host HTTP context".into(),
            });
            continue;
        }
        let Some((name, config)) = http_config(declaration)? else { continue };
        if config.get("enabled") == Some(&Value::Bool(false)) {
            continue;
        }
        let result: Result<HttpContext> = (|| {
            let _ = headers(&config)?;
            validate_oauth(&config)?;
            let root = fs::canonicalize(&plugin.manifest.root)?;
            let scope = hex::encode(Sha256::digest(serde_json::to_vec(&(
                fs::canonicalize(&plugin.source_dir)?,
                plugin.identity.as_key(),
            ))?));
            let mut context = HttpContext {
                root,
                manifest: plugin.manifest.manifest_path.clone(),
                scope,
                name: name.clone(),
                url: required_string(&config, "url")?.to_owned(),
                digest: String::new(),
            };
            context.digest = fingerprint(&context, &config)?;
            Ok(context)
        })();
        match result {
            Ok(context) => {
                if contexts.insert(name.clone(), context).is_some() {
                    outcome.errors.push(PluginMcpServerError {
                        name,
                        message: "duplicate plugin MCP HTTP credential source".into(),
                    });
                }
                match declaration {
                    PluginMcpDeclaration::Acp(McpServer::Http(server)) => server.headers.clear(),
                    PluginMcpDeclaration::Legacy { config, .. }
                    | PluginMcpDeclaration::Portable { config, .. } => {
                        for field in AUTH_FIELDS {
                            config.remove(*field);
                        }
                    }
                    _ => {}
                }
            }
            Err(error) => {
                outcome.errors.push(PluginMcpServerError { name, message: error.to_string() })
            }
        }
    }
    Ok(contexts)
}

pub(crate) fn bind(server: &mut McpServerHttp, context: HttpContext) -> Result<()> {
    server.meta.get_or_insert_default().insert(CONTEXT_KEY.into(), serde_json::to_value(context)?);
    Ok(())
}

/// Re-read the exact source declaration before connecting. Changed endpoints or packages fail closed.
pub(crate) fn runtime_headers(server: &McpServerHttp) -> Result<Option<HeaderMap>> {
    let Some(value) = server.meta.as_ref().and_then(|meta| meta.get(CONTEXT_KEY)) else {
        return Ok(None);
    };
    let context: HttpContext = serde_json::from_value(value.clone())
        .map_err(|_| anyhow::anyhow!("invalid plugin MCP HTTP source context"))?;
    if context.name != server.name || context.url != server.url || !server.headers.is_empty() {
        bail!("plugin MCP HTTP credential endpoint or identity changed");
    }
    if fs::canonicalize(&context.root)? != context.root {
        bail!("plugin MCP HTTP source root changed");
    }
    let manifest = load_plugin_manifest(&context.root)
        .context("failed to revalidate plugin MCP HTTP source")?;
    if manifest.manifest_path != context.manifest {
        bail!("plugin MCP HTTP source manifest changed");
    }
    let (outcome, _) = crate::plugins::mcp::load_plugin_mcp_declarations(&manifest)?;
    if !outcome.errors.is_empty() {
        bail!("plugin MCP HTTP declarations changed or are invalid");
    }
    let mut selected = None;
    for declaration in &outcome.servers {
        if let Some((name, config)) = http_config(declaration)?
            && name == context.name
        {
            if selected.replace(config).is_some() {
                bail!("ambiguous plugin MCP HTTP credential source");
            }
        }
    }
    let config = selected.context("plugin MCP HTTP declaration no longer exists")?;
    if fingerprint(&context, &config)? != context.digest {
        bail!("plugin MCP HTTP credential declaration changed; reload plugin configuration");
    }
    validate_oauth(&config)?;
    let headers = headers(&config)?;
    match super::mcp_public_auth::policy(&config)? {
        Some(super::mcp_public_auth::ConnectionPolicy::OAuth) => bail!(
            "plugin MCP OAuth authorization requires a separate login; interactive OAuth is not available yet"
        ),
        Some(super::mcp_public_auth::ConnectionPolicy::ApiKey) => bail!(
            "plugin MCP API-key authentication requires separate host connection credentials; package declarations cannot supply them"
        ),
        _ => {}
    }
    if headers.is_empty()
        && ["oauth", "oauth_resource", "scopes"]
            .iter()
            .any(|field| config.get(*field).is_some_and(|v| !v.is_null()))
    {
        bail!(
            "plugin MCP OAuth authorization requires a separate login; interactive OAuth is not available yet"
        );
    }
    Ok(Some(headers))
}

fn fingerprint(context: &HttpContext, config: &Map<String, Value>) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&(
        &context.root,
        &context.manifest,
        &context.scope,
        &context.name,
        &context.url,
        config,
    ))?)))
}

fn declaration_name(declaration: &PluginMcpDeclaration) -> &str {
    match declaration {
        PluginMcpDeclaration::Legacy { name, .. } | PluginMcpDeclaration::Portable { name, .. } => {
            name
        }
        PluginMcpDeclaration::Acp(McpServer::Http(server)) => &server.name,
        PluginMcpDeclaration::Acp(McpServer::Stdio(server)) => &server.name,
        PluginMcpDeclaration::Acp(McpServer::Sse(server)) => &server.name,
        _ => "unknown",
    }
}
fn forged_context(declaration: &PluginMcpDeclaration) -> bool {
    match declaration {
        PluginMcpDeclaration::Acp(McpServer::Http(server)) => {
            server.meta.as_ref().is_some_and(|m| m.contains_key(CONTEXT_KEY))
        }
        _ => false,
    }
}
fn http_config(declaration: &PluginMcpDeclaration) -> Result<Option<(String, Map<String, Value>)>> {
    match declaration {
        PluginMcpDeclaration::Legacy { name, config }
        | PluginMcpDeclaration::Portable { name, config }
            if config.contains_key("url") =>
        {
            Ok(Some((name.clone(), config.clone())))
        }
        PluginMcpDeclaration::Acp(McpServer::Http(server)) => {
            let mut headers = Map::new();
            let mut seen = HashSet::new();
            for header in &server.headers {
                if !seen.insert(header.name.to_ascii_lowercase()) {
                    bail!("duplicate plugin MCP HTTP header");
                }
                headers.insert(header.name.clone(), Value::String(header.value.clone()));
            }
            Ok(Some((
                server.name.clone(),
                Map::from_iter([
                    ("url".into(), Value::String(server.url.clone())),
                    ("http_headers".into(), Value::Object(headers)),
                ]),
            )))
        }
        _ => Ok(None),
    }
}

fn headers(config: &Map<String, Value>) -> Result<HeaderMap> {
    if config.get("bearer_token").is_some_and(|v| !v.is_null()) {
        bail!("inline plugin MCP bearer_token is forbidden; use an environment reference");
    }
    if config.get("environment_id").is_some_and(|v| !v.is_null() && v.as_str() != Some("local")) {
        bail!("remote plugin MCP credential environment is not supported");
    }
    let mut result = HeaderMap::new();
    for (field, from_env) in [("http_headers", false), ("env_http_headers", true)] {
        let Some(value) = config.get(field).filter(|v| !v.is_null()) else { continue };
        let map = value.as_object().context("plugin MCP HTTP headers must be a string map")?;
        let mut seen = HashSet::new();
        for (name, value) in map {
            let name = HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| anyhow::anyhow!("invalid plugin MCP HTTP header name"))?;
            if !seen.insert(name.clone()) {
                bail!("duplicate case-insensitive plugin MCP HTTP header");
            }
            if matches!(
                name.as_str(),
                "host"
                    | "connection"
                    | "content-length"
                    | "transfer-encoding"
                    | "mcp-session-id"
                    | "mcp-protocol-version"
                    | "proxy-authorization"
            ) {
                bail!("plugin MCP HTTP header is owned by the transport");
            }
            let raw =
                value.as_str().context("plugin MCP HTTP headers must contain string values")?;
            let value = if from_env { environment(raw)? } else { raw.to_owned() };
            result.insert(name, sensitive_header(&value)?);
        }
    }
    if let Some(value) = config.get("bearer_token_env_var").filter(|v| !v.is_null()) {
        let name = value.as_str().context("plugin MCP bearer_token_env_var must be a string")?;
        result.insert(AUTHORIZATION, sensitive_header(&format!("Bearer {}", environment(name)?))?);
    }
    validate_url(required_string(config, "url")?, !result.is_empty())?;
    Ok(result)
}
fn sensitive_header(value: &str) -> Result<HeaderValue> {
    if value.bytes().any(|b| b < 32 && b != b'\t' || b == 127) {
        bail!("invalid plugin MCP HTTP header value");
    }
    let mut value = HeaderValue::from_str(value)
        .map_err(|_| anyhow::anyhow!("invalid plugin MCP HTTP header value"))?;
    value.set_sensitive(true);
    Ok(value)
}
fn environment(name: &str) -> Result<String> {
    if name.is_empty() || name.bytes().any(|b| b == 0 || b == b'=') {
        bail!("invalid plugin MCP credential environment variable name");
    }
    std::env::var(name)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .context("required plugin MCP credential environment variable is unavailable")
}
fn required_string<'a>(config: &'a Map<String, Value>, field: &str) -> Result<&'a str> {
    config
        .get(field)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .with_context(|| format!("plugin MCP {field} must be a nonempty string"))
}
fn validate_url(raw: &str, credentials: bool) -> Result<()> {
    let url = Url::parse(raw).map_err(|_| anyhow::anyhow!("invalid plugin MCP HTTP endpoint"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        bail!("plugin MCP endpoint must be HTTP(S) without user information or fragment");
    }
    let loopback = match url.host() {
        Some(Host::Domain(host)) => host == "localhost",
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    };
    if credentials && url.scheme() == "http" && !loopback {
        bail!("plugin MCP credentials require HTTPS outside loopback");
    }
    Ok(())
}

fn validate_oauth(config: &Map<String, Value>) -> Result<()> {
    super::mcp_public_auth::policy(config)?;
    if let Some(auth) = config.get("auth").filter(|v| !v.is_null())
        && auth.as_str() != Some("oauth")
    {
        bail!(
            "plugin MCP supports public OAuth only; hosted or enterprise authentication is unavailable"
        );
    }
    if let Some(scopes) = config.get("scopes").filter(|v| !v.is_null())
        && !scopes.as_array().is_some_and(|a| a.iter().all(Value::is_string))
    {
        bail!("plugin MCP scopes must be a string array");
    }
    if let Some(resource) = config.get("oauth_resource").filter(|v| !v.is_null()) {
        validate_url(
            resource.as_str().context("plugin MCP oauth_resource must be a string")?,
            false,
        )?;
    }
    let Some(oauth) = config.get("oauth").filter(|v| !v.is_null()) else { return Ok(()) };
    let oauth = oauth.as_object().context("plugin MCP oauth must be an object")?;
    for (field, value) in oauth {
        if value.is_null()
            && matches!(
                field.as_str(),
                "client_id"
                    | "client_secret"
                    | "callback_url"
                    | "callback_port"
                    | "token_endpoint_auth_method"
                    | "authorization_server_issuer"
            )
        {
            continue;
        }
        match field.as_str() {
            "client_id" | "client_secret" | "callback_url" => {
                value.as_str().context("invalid plugin MCP OAuth string field")?;
            }
            "callback_port" if value.as_u64().is_some_and(|port| port <= u16::MAX.into()) => {}
            "token_endpoint_auth_method" | "authorization_server_issuer" | "ema_registration" => {
                bail!("plugin MCP cannot provide enterprise authentication settings")
            }
            _ => bail!("invalid or unsupported plugin MCP OAuth field"),
        }
    }
    if let Some(secret) = oauth.get("client_secret").and_then(Value::as_str) {
        if secret.trim().is_empty()
            || oauth.get("client_id").and_then(Value::as_str).is_none_or(|id| id.trim().is_empty())
        {
            bail!("plugin MCP OAuth secret must be nonempty and requires a client identifier");
        }
    }
    if let Some(callback) = oauth.get("callback_url").and_then(Value::as_str) {
        validate_url(callback, false)?;
    }
    Ok(())
}
