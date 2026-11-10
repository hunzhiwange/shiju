//! MiniMax Code MCP configuration on an authorized execution host.

use crate::{
    mcp::{
        native_file::{JsonServerEdit, JsonServerPatch, NativeFile},
        validation::{parse_native, validate_connection, validate_new_name},
    },
    providers::live::{
        commit::ProviderTransactions,
        patch_json::object_value,
        stage::{FilePatch, WholeFile},
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

const SECTION: &str = "mcpServers";
const CONNECTION_FIELDS: &[&str] = &["command", "args", "env", "url", "headers", "type"];

#[derive(Debug, PartialEq, Eq)]
pub enum McodeMcpError {
    UnsupportedTransport,
    UnsupportedField(&'static str),
    InvalidDefinition,
    CredentialUnavailable,
    File(WriteError),
    Catalog(WriteError),
    Rollback { catalog: WriteError, file: WriteError },
}

impl From<WriteError> for McodeMcpError {
    fn from(error: WriteError) -> Self {
        Self::File(error)
    }
}

pub enum ServerEdit<'a> {
    Insert(&'a McpConnectionDefinition),
    Update(&'a McpConnectionDefinition),
    /// MiniMax Code distribution revokes the native entry on disable.
    Disable,
    Remove,
}

pub struct ServerPatch<'a> {
    pub id: &'a str,
    pub edit: ServerEdit<'a>,
}

/// Host-private import candidate. The catalog owner vets credentials and ID
/// conflicts before persistence or preview.
pub struct NativeServer {
    pub connection: McpConnectionDefinition,
    pub enabled: bool,
}

pub type NativeServers = BTreeMap<String, Result<NativeServer, McodeMcpError>>;

fn file(guard: &mut AppWriteGuard) -> Result<NativeFile<'_>, WriteError> {
    if guard.app() != AppId::Mcode {
        return Err(WriteError::InvalidResource);
    }
    Ok(NativeFile::new(guard, OsStr::new("mcp.json")))
}

pub fn read(guard: &mut AppWriteGuard) -> Result<NativeServers, WriteError> {
    let Some(bytes) = file(guard)?.read()? else {
        return Ok(BTreeMap::new());
    };
    check_json(&bytes)?;
    let mut root = object_value(&bytes).map_err(|_| WriteError::InvalidResource)?;
    match root.as_object_mut().and_then(|root| root.remove(SECTION)) {
        None => Ok(BTreeMap::new()),
        Some(Value::Object(servers)) => Ok(servers
            .into_iter()
            .map(|(id, value)| {
                let server = validate_id(&id).and_then(|()| import_server(value));
                (id, server)
            })
            .collect()),
        Some(_) => Err(WriteError::InvalidResource),
    }
}

/// The executor supplies a guard for the authorized MiniMax data directory and
/// checks binding ownership and revisions in preflight. Definitions must contain
/// host-resolved values. The catalog commit must be atomic and leave the catalog
/// unchanged on error so the native snapshot can be restored.
pub fn apply(
    guard: &mut AppWriteGuard,
    tx: &ProviderTransactions,
    tenant: &TenantScope,
    change: ServerPatch<'_>,
    preflight: impl Fn() -> Result<(), WriteError>,
    commit_catalog: impl FnOnce() -> Result<(), WriteError>,
) -> Result<(), McodeMcpError> {
    validate_id(change.id)?;
    let fields;
    let edit = match change.edit {
        ServerEdit::Insert(connection) => {
            fields = project(connection)?;
            JsonServerEdit::Insert(&fields)
        }
        ServerEdit::Update(connection) => {
            fields = project(connection)?;
            JsonServerEdit::Update { set: &fields, remove: CONNECTION_FIELDS }
        }
        ServerEdit::Disable | ServerEdit::Remove => JsonServerEdit::Remove,
    };
    let native = file(guard)?;
    let before = native.read()?;
    if let Some(bytes) = &before {
        check_json(bytes)?;
    }
    let after =
        JsonServerPatch { section: SECTION, id: change.id, edit }.apply(before.as_deref())?;
    let publish = WholeFile::new(before.as_deref(), after.clone());
    native.commit(tx, tenant, before.as_deref(), &publish, preflight)?;

    if let Err(catalog) = commit_catalog() {
        if before != after {
            let restore = WholeFile::new(after.as_deref(), before);
            // Keep the guard and compare against our published bytes so rollback
            // cannot erase a subsequent external edit.
            file(guard)?
                .commit(tx, tenant, after.as_deref(), &restore, || Ok(()))
                .map_err(|file| McodeMcpError::Rollback { catalog, file })?;
        }
        return Err(McodeMcpError::Catalog(catalog));
    }
    Ok(())
}

fn check_json(bytes: &[u8]) -> Result<(), WriteError> {
    // Mcode requires strict JSON. The shared editor additionally rejects duplicate keys.
    serde_json::from_slice::<serde::de::IgnoredAny>(bytes)
        .map_err(|_| WriteError::InvalidResource)?;
    Ok(())
}

fn validate_id(id: &str) -> Result<(), McodeMcpError> {
    if !validate_new_name(id, &[]).is_ok_and(|name| name.0 == id) {
        return Err(McodeMcpError::InvalidDefinition);
    }
    Ok(())
}

fn import_server(value: Value) -> Result<NativeServer, McodeMcpError> {
    let Value::Object(mut fields) = value else {
        return Err(McodeMcpError::InvalidDefinition);
    };
    let enabled = match fields.remove("enabled") {
        None => true,
        Some(Value::Bool(enabled)) => enabled,
        Some(_) => return Err(McodeMcpError::InvalidDefinition),
    };
    match fields.get("type") {
        None => {}
        Some(Value::String(kind))
            if matches!(kind.as_str(), "stdio" | "http" | "streamable-http" | "sse") => {}
        Some(Value::String(_)) => return Err(McodeMcpError::UnsupportedTransport),
        Some(_) => return Err(McodeMcpError::InvalidDefinition),
    }
    fields.retain(|key, _| CONNECTION_FIELDS.contains(&key.as_str()));
    let connection =
        parse_native(Value::Object(fields)).map_err(|_| McodeMcpError::InvalidDefinition)?;
    Ok(NativeServer { connection, enabled })
}

fn project(connection: &McpConnectionDefinition) -> Result<Map<String, Value>, McodeMcpError> {
    validate_connection(connection).map_err(|_| McodeMcpError::InvalidDefinition)?;
    let mut fields = Map::new();
    let kind = match connection {
        McpConnectionDefinition::Stdio { command, args, env, cwd } => {
            if cwd.is_some() {
                return Err(McodeMcpError::UnsupportedField("cwd"));
            }
            fields.insert("command".into(), json!(command));
            if !args.is_empty() {
                fields.insert("args".into(), json!(args));
            }
            if !env.is_empty() {
                fields.insert("env".into(), literals(env)?);
            }
            "stdio"
        }
        McpConnectionDefinition::StreamableHttp { url, headers }
        | McpConnectionDefinition::Sse { url, headers } => {
            fields.insert("url".into(), json!(url));
            if !headers.is_empty() {
                fields.insert("headers".into(), literals(headers)?);
            }
            if matches!(connection, McpConnectionDefinition::Sse { .. }) { "sse" } else { "http" }
        }
    };
    fields.insert("type".into(), json!(kind));
    fields.insert("enabled".into(), Value::Bool(true));
    Ok(fields)
}

fn literals(values: &BTreeMap<String, McpConfigValue>) -> Result<Value, McodeMcpError> {
    values
        .iter()
        .map(|(name, value)| match value {
            McpConfigValue::Literal { value } => Ok((name.clone(), json!(value))),
            McpConfigValue::Credential { .. } => Err(McodeMcpError::CredentialUnavailable),
        })
        .collect::<Result<Map<_, _>, _>>()
        .map(Value::Object)
}
