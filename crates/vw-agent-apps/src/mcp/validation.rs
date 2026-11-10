//! Input validation for managed MCP definitions.

use reqwest::{
    Url,
    header::{HeaderName, HeaderValue},
};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fmt};
#[cfg(unix)]
use std::{os::unix::fs::PermissionsExt, path::Path};
use vw_api_types::mcp_management::{McpConfigValue, McpConnectionDefinition, McpRecordId};

/// Reserved placeholder in the JSON editor's public projection.
pub const SECRET_MASK: &str = "••••••••";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationError {
    InvalidField(&'static str),
    NameConflict,
    CommandUnavailable,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidField(field) => write!(formatter, "invalid MCP field: {field}"),
            Self::NameConflict => formatter.write_str("MCP name already exists"),
            Self::CommandUnavailable => {
                formatter.write_str("MCP command is not executable on the target host")
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Parse a native connection object into the managed connection DTO.
pub fn parse_native(input: Value) -> Result<McpConnectionDefinition, ValidationError> {
    let Value::Object(mut fields) = input else {
        return Err(ValidationError::InvalidField("connection"));
    };
    let transport = match fields.remove("type") {
        None if fields.contains_key("url") && !fields.contains_key("command") => "streamable_http",
        None => "stdio",
        Some(Value::String(value)) => match value.as_str() {
            "stdio" => "stdio",
            "http" | "streamable-http" | "streamableHttp" | "streamable_http" => "streamable_http",
            "sse" => "sse",
            _ => return Err(ValidationError::InvalidField("type")),
        },
        _ => return Err(ValidationError::InvalidField("type")),
    };
    if let Some(headers) = fields.remove("http_headers")
        && fields.insert("headers".into(), headers).is_some()
    {
        return Err(ValidationError::InvalidField("headers"));
    }
    for field in ["env", "headers"] {
        if let Some(values) = fields.get_mut(field) {
            let values = values.as_object_mut().ok_or(ValidationError::InvalidField(field))?;
            for value in values.values_mut() {
                let text = value.as_str().ok_or(ValidationError::InvalidField(field))?;
                *value = json!({ "kind": "literal", "value": text });
            }
        }
    }
    if fields.insert("transport".into(), Value::from(transport)).is_some() {
        return Err(ValidationError::InvalidField("transport"));
    }
    for field in ["command", "cwd", "url"] {
        if let Some(Value::String(value)) = fields.get_mut(field) {
            *value = value.trim().to_owned();
        }
    }
    let connection = serde_json::from_value(Value::Object(fields))
        .map_err(|_| ValidationError::InvalidField("connection"))?;
    validate_connection(&connection)?;
    Ok(connection)
}

/// Check a managed connection before persisting it or resolving any credentials.
pub fn validate_connection(connection: &McpConnectionDefinition) -> Result<(), ValidationError> {
    match connection {
        McpConnectionDefinition::Stdio { command, args, env, cwd } => {
            if !valid_trimmed_text(command) {
                return Err(ValidationError::InvalidField("command"));
            }
            if args.iter().any(|arg| arg.contains('\0')) {
                return Err(ValidationError::InvalidField("args"));
            }
            if cwd.as_deref().is_some_and(|cwd| !valid_trimmed_text(cwd)) {
                return Err(ValidationError::InvalidField("cwd"));
            }
            for (name, value) in env {
                if !valid_trimmed_text(name) || name.contains('=') {
                    return Err(ValidationError::InvalidField("env"));
                }
                validate_config_value(value, "env", |value| !value.contains('\0'))?;
            }
        }
        McpConnectionDefinition::StreamableHttp { url, headers }
        | McpConnectionDefinition::Sse { url, headers } => {
            let has_http_authority = url.split_once("://").is_some_and(|(scheme, authority)| {
                (scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
                    && !authority.starts_with('/')
            });
            if !has_http_authority
                || url.chars().any(|character| character.is_control() || character.is_whitespace())
                || url.contains('\\')
                || !Url::parse(url).is_ok_and(|url| url.has_host())
            {
                return Err(ValidationError::InvalidField("url"));
            }
            let mut seen = BTreeSet::new();
            for (name, value) in headers {
                let name = HeaderName::from_bytes(name.as_bytes())
                    .map_err(|_| ValidationError::InvalidField("headers"))?;
                if !seen.insert(name.as_str().to_owned()) {
                    return Err(ValidationError::InvalidField("headers"));
                }
                validate_config_value(value, "headers", |value| {
                    HeaderValue::from_bytes(value.as_bytes()).is_ok()
                })?;
            }
        }
    }
    Ok(())
}

fn valid_trimmed_text(value: &str) -> bool {
    !value.is_empty() && value == value.trim() && !value.chars().any(char::is_control)
}

fn validate_config_value(
    value: &McpConfigValue,
    field: &'static str,
    valid_literal: impl FnOnce(&str) -> bool,
) -> Result<(), ValidationError> {
    let valid = match value {
        McpConfigValue::Literal { value } => value.trim() != SECRET_MASK && valid_literal(value),
        McpConfigValue::Credential { reference } => {
            valid_trimmed_text(&reference.account_id) && reference.account_id != SECRET_MASK
        }
    };
    if !valid {
        return Err(ValidationError::InvalidField(field));
    }
    Ok(())
}

/// Check a new ID against records in the caller's authorized catalog scope.
/// The storage write still needs to enforce uniqueness atomically.
pub fn validate_new_name(
    name: &str,
    existing: &[McpRecordId],
) -> Result<McpRecordId, ValidationError> {
    let name = name.trim();
    if !valid_trimmed_text(name) || name.chars().any(char::is_whitespace) {
        return Err(ValidationError::InvalidField("name"));
    }
    if existing.iter().any(|id| id.0 == name) {
        return Err(ValidationError::NameConflict);
    }
    Ok(McpRecordId(name.into()))
}

/// Check a program path supplied by the authorized target host's resolver.
/// Reads file metadata only. Call this on that host before launching stdio.
#[cfg(unix)]
pub fn validate_resolved_command(path: Option<&Path>) -> Result<&Path, ValidationError> {
    let path = path.filter(|path| path.is_absolute()).ok_or(ValidationError::CommandUnavailable)?;
    let metadata = path.metadata().map_err(|_| ValidationError::CommandUnavailable)?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return Err(ValidationError::CommandUnavailable);
    }
    Ok(path)
}
