//! Pi's user-level mcp.json on the authorized execution host.

use crate::{
    mcp::{
        native_file::{JsonServerEdit, JsonServerPatch, NativeFile},
        validation::{parse_native, validate_connection},
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
const TRANSPORT_FIELDS: &[&str] = &["command", "args", "env", "cwd", "url", "headers", "type"];
const PI_OPTIONS: &[&str] =
    &["timeout", "description", "exposure", "toolExposure", "oauth", "auth"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiMcpError {
    InvalidDefinition,
    UnsupportedTransport,
    CredentialUnavailable,
    File(WriteError),
}

impl From<WriteError> for PiMcpError {
    fn from(error: WriteError) -> Self {
        Self::File(error)
    }
}

/// A host-private import candidate. The catalog owner must vet connection
/// values and native options before storage or display.
pub struct PiMcpServer {
    pub connection: McpConnectionDefinition,
    /// On read, this is the native file state. The catalog retains its own desired state.
    pub enabled: bool,
    pub options: Map<String, Value>,
}

pub enum PiMcpEdit<'a> {
    /// An existing Pi name conflicts with a new binding.
    Insert(&'a PiMcpServer),
    /// Replace the verified binding's connection while retaining its enabled state.
    Update(&'a McpConnectionDefinition),
    /// Replace the connection and enable the binding in the same transaction.
    Enable(&'a McpConnectionDefinition),
    /// Explicitly change the verified binding's native enabled flag.
    SetEnabled(bool),
    /// Delete the verified binding.
    Remove,
}

pub struct PiMcp<'a>(NativeFile<'a>);

impl<'a> PiMcp<'a> {
    /// Use the guard from the authorized Pi installation's agent directory.
    pub fn new(guard: &'a mut AppWriteGuard) -> Result<Self, WriteError> {
        if guard.app() != AppId::Pi {
            return Err(WriteError::InvalidResource);
        }
        Ok(Self(NativeFile::new(guard, OsStr::new("mcp.json"))))
    }

    pub fn read(&self) -> Result<BTreeMap<String, Result<PiMcpServer, PiMcpError>>, WriteError> {
        let bytes = self.0.read()?;
        let servers = server_map(bytes.as_deref())?;
        let mut names = BTreeMap::new();
        for id in servers.keys() {
            *names.entry(normalized_name(id)).or_insert(0_usize) += 1;
        }
        Ok(servers
            .into_iter()
            .map(|(id, value)| {
                let server = validate_name(&id).and_then(|()| {
                    if names[&normalized_name(&id)] > 1 {
                        return Err(WriteError::Conflict.into());
                    }
                    import_server(value)
                });
                (id, server)
            })
            .collect())
    }

    /// Connection credentials must be resolved on the execution host.
    /// Preflight rechecks ownership and the saved revision before publication.
    pub fn apply(
        self,
        transactions: &ProviderTransactions,
        tenant: &TenantScope,
        id: &str,
        edit: PiMcpEdit<'_>,
        preflight: impl Fn() -> Result<(), WriteError>,
    ) -> Result<(), PiMcpError> {
        validate_name(id)?;
        let expected = self.0.read()?;
        let mut servers = server_map(expected.as_deref())?;
        if matches!(
            edit,
            PiMcpEdit::Insert(_)
                | PiMcpEdit::Update(_)
                | PiMcpEdit::Enable(_)
                | PiMcpEdit::SetEnabled(true)
        ) {
            let name = normalized_name(id);
            if servers.keys().any(|key| key != id && normalized_name(key) == name) {
                return Err(WriteError::Conflict.into());
            }
        }

        let fields;
        let remove;
        let edit = match edit {
            PiMcpEdit::Insert(server) => {
                let mut projected = project_connection(&server.connection)?;
                projected.extend(native_options(&server.options));
                projected.insert("enabled".into(), json!(server.enabled));
                fields = projected;
                JsonServerEdit::Insert(&fields)
            }
            PiMcpEdit::Update(connection) | PiMcpEdit::Enable(connection) => {
                let mut projected = project_connection(connection)?;
                if matches!(edit, PiMcpEdit::Enable(_)) {
                    projected.insert("enabled".into(), json!(true));
                }
                fields = projected;
                remove = TRANSPORT_FIELDS
                    .iter()
                    .copied()
                    .filter(|key| !fields.contains_key(*key))
                    .collect::<Vec<_>>();
                JsonServerEdit::Update { set: &fields, remove: &remove }
            }
            PiMcpEdit::SetEnabled(enabled) => {
                match servers.remove(id) {
                    Some(value) if enabled => {
                        import_server(value)?;
                    }
                    None if !enabled => return Ok(()),
                    _ => {}
                }
                fields = Map::from_iter([("enabled".into(), json!(enabled))]);
                JsonServerEdit::Update { set: &fields, remove: &[] }
            }
            PiMcpEdit::Remove => JsonServerEdit::Remove,
        };
        let patch = JsonServerPatch { section: SERVERS, id, edit };
        self.0.commit(transactions, tenant, expected.as_deref(), &patch, preflight)?;
        Ok(())
    }
}

fn server_map(snapshot: Option<&[u8]>) -> Result<Map<String, Value>, WriteError> {
    let mut root =
        object_value(snapshot.unwrap_or(b"{}")).map_err(|_| WriteError::InvalidResource)?;
    match root.as_object_mut().and_then(|root| root.remove(SERVERS)) {
        None => Ok(Map::new()),
        Some(Value::Object(servers)) => Ok(servers),
        Some(_) => Err(WriteError::InvalidResource),
    }
}

fn validate_name(id: &str) -> Result<(), PiMcpError> {
    if id.is_empty()
        || !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
    {
        return Err(PiMcpError::InvalidDefinition);
    }
    Ok(())
}

fn normalized_name(id: &str) -> String {
    id.replace('-', "_")
}

fn native_options(fields: &Map<String, Value>) -> Map<String, Value> {
    PI_OPTIONS
        .iter()
        .filter_map(|&key| fields.get(key).map(|value| (key.to_owned(), value.clone())))
        .collect()
}

fn import_server(value: Value) -> Result<PiMcpServer, PiMcpError> {
    let Value::Object(mut fields) = value else {
        return Err(PiMcpError::InvalidDefinition);
    };
    let enabled = match fields.get("enabled") {
        None => true,
        Some(value) => value.as_bool().ok_or(PiMcpError::InvalidDefinition)?,
    };
    let options = native_options(&fields);
    match fields.get("type") {
        None => {}
        Some(Value::String(kind))
            if matches!(kind.as_str(), "stdio" | "http" | "streamable-http") => {}
        Some(Value::String(_)) => return Err(PiMcpError::UnsupportedTransport),
        Some(_) => return Err(PiMcpError::InvalidDefinition),
    }
    fields.retain(|key, _| TRANSPORT_FIELDS.contains(&key.as_str()));
    let connection =
        parse_native(Value::Object(fields)).map_err(|_| PiMcpError::InvalidDefinition)?;
    Ok(PiMcpServer { connection, enabled, options })
}

fn project_connection(
    connection: &McpConnectionDefinition,
) -> Result<Map<String, Value>, PiMcpError> {
    validate_connection(connection).map_err(|_| PiMcpError::InvalidDefinition)?;
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
        McpConnectionDefinition::StreamableHttp { url, headers } => {
            fields.insert("type".into(), json!("http"));
            fields.insert("url".into(), json!(url));
            if !headers.is_empty() {
                fields.insert("headers".into(), Value::Object(literal_fields(headers)?));
            }
        }
        McpConnectionDefinition::Sse { .. } => return Err(PiMcpError::UnsupportedTransport),
    }
    Ok(fields)
}

fn literal_fields(
    values: &BTreeMap<String, McpConfigValue>,
) -> Result<Map<String, Value>, PiMcpError> {
    values
        .iter()
        .map(|(key, value)| match value {
            McpConfigValue::Literal { value } => Ok((key.clone(), json!(value))),
            McpConfigValue::Credential { .. } => Err(PiMcpError::CredentialUnavailable),
        })
        .collect()
}
