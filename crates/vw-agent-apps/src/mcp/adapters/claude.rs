//! Claude MCP projection into an explicitly authorized managed account profile.

use crate::{
    auth::credential_lease::CredentialLease,
    mcp::{
        native_file::{JsonServerEdit, JsonServerPatch, NativeFile},
        validation::{parse_native, validate_connection, validate_new_name},
    },
    providers::live::{
        commit::ProviderTransactions,
        patch_json::object_value,
        write_guard::{WriteError, lock_directory},
    },
};
use serde_json::{Map, Value, json};
use std::{collections::BTreeMap, ffi::OsStr};
use vw_api_types::{
    agent_apps::AppId,
    mcp_management::{McpConfigValue, McpConnectionDefinition},
};

const PROFILE: &str = ".claude.json";
const SERVERS: &str = "mcpServers";
const CONNECTION_FIELDS: &[&str] =
    &["type", "transport", "command", "args", "env", "cwd", "url", "headers", "http_headers"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaudeMcpError {
    UnsupportedTransport,
    InvalidDefinition,
    CredentialUnavailable,
    File(WriteError),
}

impl From<WriteError> for ClaudeMcpError {
    fn from(error: WriteError) -> Self {
        Self::File(error)
    }
}

pub enum ClaudeMcpEdit<'a> {
    Insert(&'a McpConnectionDefinition),
    /// Update a binding whose ownership the executor has verified.
    Update(&'a McpConnectionDefinition),
    /// Disable or delete a binding whose ownership the executor has verified.
    Remove,
}

/// Host-private import candidates. Apply catalog credential policy before
/// persisting or exposing definitions.
pub fn read(
    account: &CredentialLease,
) -> Result<BTreeMap<String, Result<McpConnectionDefinition, ClaudeMcpError>>, WriteError> {
    let mut guard = lock_directory(AppId::Claude, account.open_claude_profile()?.0)?;
    let Some(bytes) = NativeFile::new(&mut guard, OsStr::new(PROFILE)).read()? else {
        return Ok(BTreeMap::new());
    };
    let mut root = object_value(&bytes).map_err(|_| WriteError::InvalidResource)?;
    match root.as_object_mut().and_then(|root| root.remove(SERVERS)) {
        None => Ok(BTreeMap::new()),
        Some(Value::Object(servers)) => Ok(servers
            .into_iter()
            .map(|(id, entry)| {
                let definition = validate_id(&id).and_then(|()| import_connection(entry));
                (id, definition)
            })
            .collect()),
        Some(_) => Err(WriteError::InvalidResource),
    }
}

/// Mirror one managed catalog entry into one authorized account. The executor
/// resolves credentials first, acquires each account lease separately and
/// rechecks binding ownership and catalog revision in preflight.
pub fn apply(
    account: &CredentialLease,
    transactions: &ProviderTransactions,
    id: &str,
    edit: ClaudeMcpEdit<'_>,
    preflight: impl Fn() -> Result<(), WriteError>,
) -> Result<(), ClaudeMcpError> {
    let (tenant, host) = account.claude_profile_scope()?;
    if !transactions.is_for_host(host) {
        return Err(WriteError::InvalidResource.into());
    }
    validate_id(id)?;
    let fields;
    let remove;
    let edit = match edit {
        ClaudeMcpEdit::Insert(connection) => {
            fields = project_connection(connection)?;
            JsonServerEdit::Insert(&fields)
        }
        ClaudeMcpEdit::Update(connection) => {
            fields = project_connection(connection)?;
            remove = CONNECTION_FIELDS
                .iter()
                .copied()
                .filter(|key| !fields.contains_key(*key))
                .collect::<Vec<_>>();
            JsonServerEdit::Update { set: &fields, remove: &remove }
        }
        ClaudeMcpEdit::Remove => JsonServerEdit::Remove,
    };

    let directory = account.open_claude_profile()?;
    let _cli_lock = directory.cli_lock(".claude.json.lock")?;
    let mut guard =
        lock_directory(AppId::Claude, directory.0.try_clone().map_err(|_| WriteError::Io)?)?;
    account.verify_claude_profile(&directory.0)?;
    let file = NativeFile::new(&mut guard, OsStr::new(PROFILE));
    let expected = file.read()?;
    let patch = JsonServerPatch { section: SERVERS, id, edit };
    file.commit(transactions, tenant, expected.as_deref(), &patch, || {
        preflight()?;
        account.verify_claude_profile(&directory.0)
    })?;
    Ok(())
}

fn validate_id(id: &str) -> Result<(), ClaudeMcpError> {
    if validate_new_name(id, &[]).is_ok_and(|name| name.0 == id) {
        Ok(())
    } else {
        Err(ClaudeMcpError::InvalidDefinition)
    }
}

fn import_connection(value: Value) -> Result<McpConnectionDefinition, ClaudeMcpError> {
    let Value::Object(mut fields) = value else {
        return Err(ClaudeMcpError::InvalidDefinition);
    };
    if let Some(transport) = fields.get("type").and_then(Value::as_str)
        && !matches!(transport, "stdio" | "http" | "sse")
    {
        return Err(ClaudeMcpError::UnsupportedTransport);
    }
    // Client extensions stay in the native file. The catalog receives only
    // the connection fields it can represent.
    fields.retain(|key, _| CONNECTION_FIELDS.contains(&key.as_str()));
    parse_native(Value::Object(fields)).map_err(|_| ClaudeMcpError::InvalidDefinition)
}

fn project_connection(
    connection: &McpConnectionDefinition,
) -> Result<Map<String, Value>, ClaudeMcpError> {
    validate_connection(connection).map_err(|_| ClaudeMcpError::InvalidDefinition)?;
    let mut fields = Map::new();
    match connection {
        McpConnectionDefinition::Stdio { command, args, env, cwd } => {
            fields.insert("type".into(), json!("stdio"));
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
            let transport = match connection {
                McpConnectionDefinition::StreamableHttp { .. } => "http",
                _ => "sse",
            };
            fields.insert("type".into(), json!(transport));
            fields.insert("url".into(), json!(url));
            if !headers.is_empty() {
                fields.insert("headers".into(), Value::Object(literal_fields(headers)?));
            }
        }
    }
    Ok(fields)
}

fn literal_fields(
    values: &BTreeMap<String, McpConfigValue>,
) -> Result<Map<String, Value>, ClaudeMcpError> {
    values
        .iter()
        .map(|(name, value)| match value {
            McpConfigValue::Literal { value } => Ok((name.clone(), json!(value))),
            McpConfigValue::Credential { .. } => Err(ClaudeMcpError::CredentialUnavailable),
        })
        .collect()
}
