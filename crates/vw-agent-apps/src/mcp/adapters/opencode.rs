//! Managed OpenCode MCP declarations.
use crate::{
    mcp::{
        native_file::{JsonServerEdit, JsonServerPatch, NativeFile},
        validation::{validate_connection, validate_new_name},
    },
    providers::{
        live::{commit::ProviderTransactions, patch_json::object_value, write_guard::WriteError},
        opencode::{self, OpenCodeFiles},
    },
};
use serde_json::{Map, Value, json};
use std::{collections::BTreeMap, ffi::OsStr, path::Path};
use vw_api_types::{
    TenantScope,
    mcp_management::{McpConfigValue, McpConnectionDefinition},
};

const SECTION: &str = "mcp";
const CONNECTION_FIELDS: &[&str] = &["type", "command", "environment", "url", "headers"];

#[derive(Debug, PartialEq, Eq)]
pub enum OpenCodeMcpError {
    UnsupportedTransport,
    UnsupportedField(&'static str),
    InvalidDefinition,
    CredentialUnavailable,
    File(WriteError),
}

impl From<WriteError> for OpenCodeMcpError {
    fn from(error: WriteError) -> Self {
        Self::File(error)
    }
}

pub enum Edit<'a> {
    /// The connection contains only host-resolved values.
    Insert(&'a McpConnectionDefinition),
    /// Replace the connection of a binding verified by the executor.
    Update(&'a McpConnectionDefinition),
    /// Change the enabled flag of a binding verified by the executor.
    SetEnabled(bool),
    /// Revoke a binding verified by the executor.
    Remove,
}

/// Host-private import candidate. Native fields can contain credentials.
pub struct ImportedServer {
    pub connection: McpConnectionDefinition,
    pub enabled: bool,
}

/// Read each declaration independently, leaving credentials and errors on the host.
pub fn read(
    home: &Path,
) -> Result<BTreeMap<String, Result<ImportedServer, OpenCodeMcpError>>, WriteError> {
    let Some(bytes) = opencode::read(home)? else {
        return Ok(BTreeMap::new());
    };
    let mut root = object_value(&bytes).map_err(|_| WriteError::InvalidResource)?;
    match root.as_object_mut().and_then(|root| root.remove(SECTION)) {
        None => Ok(BTreeMap::new()),
        Some(Value::Object(servers)) => Ok(servers
            .into_iter()
            .map(|(id, entry)| {
                let server = validate_id(&id).and_then(|()| import(entry));
                (id, server)
            })
            .collect()),
        Some(_) => Err(WriteError::InvalidResource),
    }
}

/// Publish under the same application and directory locks as provider edits.
/// The host verifies the app instance, binding ownership and revision in preflight.
pub fn apply(
    files: &mut OpenCodeFiles,
    tx: &ProviderTransactions,
    tenant: &TenantScope,
    id: &str,
    edit: Edit<'_>,
    preflight: impl Fn() -> Result<(), WriteError>,
) -> Result<(), OpenCodeMcpError> {
    validate_id(id)?;
    let mut fields;
    let edit = match edit {
        Edit::Insert(connection) => {
            fields = project(connection)?;
            fields.insert("enabled".into(), Value::Bool(true));
            JsonServerEdit::Insert(&fields)
        }
        Edit::Update(connection) => {
            fields = project(connection)?;
            JsonServerEdit::Update { set: &fields, remove: CONNECTION_FIELDS }
        }
        Edit::SetEnabled(enabled) => {
            fields = Map::from_iter([("enabled".into(), Value::Bool(enabled))]);
            JsonServerEdit::Update { set: &fields, remove: &[] }
        }
        Edit::Remove => JsonServerEdit::Remove,
    };
    let (name, expected) = files.snapshot()?;
    let check_selection = files.selected_file_preflight(name)?;
    let patch = JsonServerPatch { section: SECTION, id, edit };
    NativeFile::new(&mut files.guard, OsStr::new(name)).commit(
        tx,
        tenant,
        expected.as_deref(),
        &patch,
        || {
            preflight()?;
            check_selection()
        },
    )?;
    Ok(())
}

fn validate_id(id: &str) -> Result<(), OpenCodeMcpError> {
    let validated = validate_new_name(id, &[]).map_err(|_| OpenCodeMcpError::InvalidDefinition)?;
    if validated.0 != id {
        return Err(OpenCodeMcpError::InvalidDefinition);
    }
    Ok(())
}

fn project(connection: &McpConnectionDefinition) -> Result<Map<String, Value>, OpenCodeMcpError> {
    validate_connection(connection).map_err(|_| OpenCodeMcpError::InvalidDefinition)?;
    let fields = match connection {
        McpConnectionDefinition::Stdio { command, args, env, cwd } => {
            if cwd.is_some() {
                return Err(OpenCodeMcpError::UnsupportedField("cwd"));
            }
            let command = std::iter::once(command).chain(args).collect::<Vec<_>>();
            let mut fields = Map::from_iter([
                ("type".into(), json!("local")),
                ("command".into(), json!(command)),
            ]);
            if !env.is_empty() {
                fields.insert("environment".into(), Value::Object(project_values(env)?));
            }
            fields
        }
        McpConnectionDefinition::Sse { url, headers } => {
            let mut fields =
                Map::from_iter([("type".into(), json!("remote")), ("url".into(), json!(url))]);
            if !headers.is_empty() {
                fields.insert("headers".into(), Value::Object(project_values(headers)?));
            }
            fields
        }
        // The source imports remote as SSE. It cannot round-trip a distinct HTTP transport.
        McpConnectionDefinition::StreamableHttp { .. } => {
            return Err(OpenCodeMcpError::UnsupportedTransport);
        }
    };
    Ok(fields)
}

fn project_values(
    values: &BTreeMap<String, McpConfigValue>,
) -> Result<Map<String, Value>, OpenCodeMcpError> {
    values
        .iter()
        .map(|(name, field)| match field {
            McpConfigValue::Literal { value } => Ok((name.clone(), json!(value))),
            McpConfigValue::Credential { .. } => Err(OpenCodeMcpError::CredentialUnavailable),
        })
        .collect()
}

fn import(entry: Value) -> Result<ImportedServer, OpenCodeMcpError> {
    use OpenCodeMcpError::{InvalidDefinition, UnsupportedField, UnsupportedTransport};

    let Value::Object(mut fields) = entry else {
        return Err(InvalidDefinition);
    };
    let enabled = match fields.remove("enabled") {
        None => true,
        Some(Value::Bool(enabled)) => enabled,
        Some(_) => return Err(InvalidDefinition),
    };
    let transport = fields.remove("type");
    let connection = match transport.as_ref().map(Value::as_str) {
        None | Some(Some("local")) => {
            let command = fields.remove("command").ok_or(InvalidDefinition)?;
            let mut command = serde_json::from_value::<Vec<String>>(command)
                .map_err(|_| InvalidDefinition)?
                .into_iter();
            McpConnectionDefinition::Stdio {
                command: command.next().ok_or(InvalidDefinition)?,
                args: command.collect(),
                env: import_values(fields.remove("environment"))?,
                cwd: None,
            }
        }
        Some(Some("remote")) => McpConnectionDefinition::Sse {
            url: serde_json::from_value(fields.remove("url").ok_or(InvalidDefinition)?)
                .map_err(|_| InvalidDefinition)?,
            headers: import_values(fields.remove("headers"))?,
        },
        Some(Some(_)) => return Err(UnsupportedTransport),
        Some(None) => return Err(InvalidDefinition),
    };
    if !fields.is_empty() {
        return Err(UnsupportedField("native options"));
    }
    validate_connection(&connection).map_err(|_| InvalidDefinition)?;
    Ok(ImportedServer { connection, enabled })
}

fn import_values(
    values: Option<Value>,
) -> Result<BTreeMap<String, McpConfigValue>, OpenCodeMcpError> {
    let Some(values) = values else {
        return Ok(BTreeMap::new());
    };
    let values = serde_json::from_value::<BTreeMap<String, String>>(values)
        .map_err(|_| OpenCodeMcpError::InvalidDefinition)?;
    Ok(values.into_iter().map(|(name, value)| (name, McpConfigValue::Literal { value })).collect())
}

#[cfg(test)]
#[path = "opencode_tests.rs"]
mod tests;
