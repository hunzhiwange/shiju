//! Kimi Code MCP configuration under the authorized app directory.

use crate::{
    mcp::{
        native_file::{JsonServerEdit, JsonServerPatch, NativeFile},
        validation::{ValidationError, parse_native, validate_connection, validate_new_name},
    },
    providers::live::{
        commit::ProviderTransactions,
        patch_json::object_value,
        write_guard::{AppWriteGuard, WriteError},
    },
};
use serde_json::{Map, Value, json};
use std::{collections::BTreeMap, ffi::OsStr};
use vw_api_types::{
    TenantScope,
    agent_apps::AppId,
    mcp_management::{McpConfigValue, McpConnectionDefinition},
};

const SERVERS: &str = "mcpServers";
const CONNECTION_FIELDS: &[&str] =
    &["type", "transport", "command", "args", "env", "cwd", "url", "headers", "http_headers"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KimiMcpError {
    UnsupportedTransport,
    InvalidDefinition,
    CredentialUnavailable,
    File(WriteError),
}

impl From<WriteError> for KimiMcpError {
    fn from(error: WriteError) -> Self {
        Self::File(error)
    }
}

pub enum Edit<'a> {
    /// Publish a new binding. An existing same-name service conflicts.
    Insert(&'a McpConnectionDefinition),
    /// Replace the connection of a binding whose ownership the executor verified.
    Update(&'a McpConnectionDefinition),
    /// Disable or delete a verified binding. An absent entry is already revoked.
    Remove,
}

/// Host-private import candidate. The definition may contain credentials.
pub struct ImportedServer {
    pub connection: McpConnectionDefinition,
    pub enabled: bool,
}

fn native_file(guard: &mut AppWriteGuard) -> Result<NativeFile<'_>, WriteError> {
    if guard.app() != AppId::Kimi {
        return Err(WriteError::InvalidResource);
    }
    Ok(NativeFile::new(guard, OsStr::new("mcp.json")))
}

/// Return per-entry import results for the authorized Kimi configuration.
/// Candidates must pass catalog credential policy before storage or exposure.
pub fn read(
    guard: &mut AppWriteGuard,
) -> Result<BTreeMap<String, Result<ImportedServer, KimiMcpError>>, WriteError> {
    let Some(bytes) = native_file(guard)?.read()? else {
        return Ok(BTreeMap::new());
    };
    validate_json(&bytes)?;
    let mut root = object_value(&bytes).map_err(|_| WriteError::InvalidResource)?;
    match root.as_object_mut().and_then(|root| root.remove(SERVERS)) {
        None => Ok(BTreeMap::new()),
        Some(Value::Object(servers)) => Ok(servers
            .into_iter()
            .map(|(id, entry)| {
                let server = validate_id(&id).and_then(|()| import_entry(entry));
                (id, server)
            })
            .collect()),
        Some(_) => Err(WriteError::InvalidResource),
    }
}

/// Publish through the same directory lock and transaction as provider writes.
/// The executor supplies resolved credentials and rechecks ownership and revision in preflight.
pub fn apply(
    guard: &mut AppWriteGuard,
    tx: &ProviderTransactions,
    tenant: &TenantScope,
    id: &str,
    edit: Edit<'_>,
    preflight: impl Fn() -> Result<(), WriteError>,
) -> Result<(), KimiMcpError> {
    let file = native_file(guard)?;
    validate_id(id)?;
    let fields;
    let edit = match edit {
        Edit::Insert(connection) => {
            fields = project(connection)?;
            JsonServerEdit::Insert(&fields)
        }
        Edit::Update(connection) => {
            fields = project(connection)?;
            JsonServerEdit::Update { set: &fields, remove: CONNECTION_FIELDS }
        }
        Edit::Remove => JsonServerEdit::Remove,
    };
    let expected = file.read()?;
    if let Some(bytes) = expected.as_deref() {
        validate_json(bytes)?;
    }
    let patch = JsonServerPatch { section: SERVERS, id, edit };
    file.commit(tx, tenant, expected.as_deref(), &patch, preflight)?;
    Ok(())
}

fn validate_json(bytes: &[u8]) -> Result<(), WriteError> {
    // Kimi requires strict JSON. The shared JSON/JSONC editor also checks duplicate keys.
    serde_json::from_slice::<Map<String, Value>>(bytes)
        .map(|_| ())
        .map_err(|_| WriteError::InvalidResource)
}

fn validate_id(id: &str) -> Result<(), KimiMcpError> {
    if validate_new_name(id, &[]).is_ok_and(|name| name.0 == id) {
        Ok(())
    } else {
        Err(KimiMcpError::InvalidDefinition)
    }
}

fn project(connection: &McpConnectionDefinition) -> Result<Map<String, Value>, KimiMcpError> {
    validate_connection(connection).map_err(|_| KimiMcpError::InvalidDefinition)?;
    let mut fields = Map::new();
    match connection {
        McpConnectionDefinition::Stdio { command, args, env, cwd } => {
            fields.insert("command".into(), json!(command));
            if !args.is_empty() {
                fields.insert("args".into(), json!(args));
            }
            if !env.is_empty() {
                fields.insert("env".into(), Value::Object(literal_fields(env)?));
            }
            if let Some(cwd) = cwd {
                fields.insert("cwd".into(), json!(cwd));
            }
        }
        McpConnectionDefinition::StreamableHttp { url, headers }
        | McpConnectionDefinition::Sse { url, headers } => {
            fields.insert("url".into(), json!(url));
            if !headers.is_empty() {
                fields.insert("headers".into(), Value::Object(literal_fields(headers)?));
            }
            if matches!(connection, McpConnectionDefinition::Sse { .. }) {
                fields.insert("transport".into(), json!("sse"));
            }
        }
    }
    fields.insert("enabled".into(), Value::Bool(true));
    Ok(fields)
}

fn literal_fields(
    values: &BTreeMap<String, McpConfigValue>,
) -> Result<Map<String, Value>, KimiMcpError> {
    values
        .iter()
        .map(|(name, value)| match value {
            McpConfigValue::Literal { value } => Ok((name.clone(), json!(value))),
            McpConfigValue::Credential { .. } => Err(KimiMcpError::CredentialUnavailable),
        })
        .collect()
}

fn import_entry(entry: Value) -> Result<ImportedServer, KimiMcpError> {
    let Value::Object(mut fields) = entry else {
        return Err(KimiMcpError::InvalidDefinition);
    };
    let enabled = match fields.remove("enabled") {
        None => true,
        Some(Value::Bool(enabled)) => enabled,
        Some(_) => return Err(KimiMcpError::InvalidDefinition),
    };
    fields.retain(|key, _| CONNECTION_FIELDS.contains(&key.as_str()));
    if let Some(transport) = fields.remove("transport") {
        fields.insert("type".into(), transport);
    }
    if fields.get("type").is_some_and(|kind| !kind.is_string()) {
        return Err(KimiMcpError::InvalidDefinition);
    }
    let connection = parse_native(Value::Object(fields)).map_err(|error| match error {
        ValidationError::InvalidField("type") => KimiMcpError::UnsupportedTransport,
        _ => KimiMcpError::InvalidDefinition,
    })?;
    Ok(ImportedServer { connection, enabled })
}
