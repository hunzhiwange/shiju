//! Oh My Pi user-level MCP projection on an authorized execution host.

use crate::{
    mcp::{
        native_file::NativeFile,
        validation::{parse_native, validate_connection, validate_new_name},
    },
    providers::live::{
        commit::ProviderTransactions,
        patch_json::{object_value, replace_object_preserving_trivia},
        stage::FilePatch,
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
const ALLOW: &str = "enabledServers";
const DENY: &str = "disabledServers";
// The frozen OMP adapter uses the same connection fields as Pi.
const CONNECTION_FIELDS: &[&str] = &["type", "command", "args", "env", "cwd", "url", "headers"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OmpMcpError {
    UnsupportedTransport,
    InvalidDefinition,
    CredentialUnavailable,
    File(WriteError),
}

impl From<WriteError> for OmpMcpError {
    fn from(error: WriteError) -> Self {
        Self::File(error)
    }
}

pub enum OmpMcpEdit<'a> {
    /// Publish a new binding. An existing same-name entry or policy conflicts.
    Insert(&'a McpConnectionDefinition),
    /// Replace connection fields and set the desired state of a verified binding.
    Update { connection: &'a McpConnectionDefinition, enabled: bool },
    /// Suppress a verified binding and any discoveries of the same name.
    Disable,
    /// Remove a verified binding and its policy entries.
    Remove,
}

pub struct OmpMcpEntry {
    pub connection: McpConnectionDefinition,
    pub enabled: bool,
}

pub struct OmpMcp<'a>(NativeFile<'a>);

impl<'a> OmpMcp<'a> {
    /// The guard must cover the authorized installation's OMP agent directory.
    pub fn new(guard: &'a mut AppWriteGuard) -> Result<Self, WriteError> {
        if guard.app() != AppId::Omp {
            return Err(WriteError::InvalidResource);
        }
        Ok(Self(NativeFile::new(guard, OsStr::new("mcp.json"))))
    }

    /// Return host-private import candidates. Apply catalog credential policy
    /// before persisting or exposing connection definitions.
    pub fn read(&self) -> Result<BTreeMap<String, Result<OmpMcpEntry, OmpMcpError>>, WriteError> {
        let Some(bytes) = self.0.read()? else {
            return Ok(BTreeMap::new());
        };
        let root = read_document(&bytes)?;
        let entries = root[SERVERS].as_object();
        Ok(entries
            .into_iter()
            .flatten()
            .map(|(id, entry)| (id.clone(), import_entry(&root, id, entry)))
            .collect())
    }

    /// The executor resolves credentials on the host and rechecks binding
    /// ownership and catalog revision in preflight.
    pub fn apply(
        self,
        transactions: &ProviderTransactions,
        tenant: &TenantScope,
        id: &str,
        edit: OmpMcpEdit<'_>,
        preflight: impl Fn() -> Result<(), WriteError>,
    ) -> Result<(), OmpMcpError> {
        validate_id(id)?;
        let edit = match edit {
            OmpMcpEdit::Insert(connection) => NativeEdit::Insert(project(connection)?),
            OmpMcpEdit::Update { connection, enabled } => {
                NativeEdit::Update { fields: project(connection)?, enabled }
            }
            OmpMcpEdit::Disable => NativeEdit::Disable,
            OmpMcpEdit::Remove => NativeEdit::Remove,
        };
        let before = self.0.read()?;
        self.0.commit(
            transactions,
            tenant,
            before.as_deref(),
            &OmpPatch { id, edit },
            preflight,
        )?;
        Ok(())
    }
}

enum NativeEdit {
    Insert(Map<String, Value>),
    Update { fields: Map<String, Value>, enabled: bool },
    Disable,
    Remove,
}

struct OmpPatch<'a> {
    id: &'a str,
    edit: NativeEdit,
}

impl FilePatch for OmpPatch<'_> {
    fn apply(&self, before: Option<&[u8]>) -> Result<Option<Vec<u8>>, WriteError> {
        let source = before.unwrap_or(b"{}");
        let mut root = read_document(source)?;
        let has_policy = listed(&root, ALLOW, self.id) || listed(&root, DENY, self.id);
        let entry = root.get_mut(SERVERS).and_then(|servers| servers.get_mut(self.id));
        let deny = match &self.edit {
            NativeEdit::Insert(fields) => {
                if entry.is_some() || has_policy {
                    return Err(WriteError::Conflict);
                }
                let mut fields = fields.clone();
                fields.insert("enabled".into(), Value::Bool(true));
                root[SERVERS][self.id] = Value::Object(fields);
                false
            }
            NativeEdit::Update { fields, enabled } => {
                let entry = entry
                    .ok_or(WriteError::Conflict)?
                    .as_object_mut()
                    .ok_or(WriteError::InvalidResource)?;
                entry.retain(|key, _| !CONNECTION_FIELDS.contains(&key.as_str()));
                entry.extend(fields.clone());
                entry.insert("enabled".into(), Value::Bool(*enabled));
                !enabled
            }
            NativeEdit::Disable => {
                if let Some(entry) = entry {
                    entry
                        .as_object_mut()
                        .ok_or(WriteError::InvalidResource)?
                        .insert("enabled".into(), Value::Bool(false));
                }
                true
            }
            NativeEdit::Remove => {
                if let Some(servers) = root.get_mut(SERVERS).and_then(Value::as_object_mut) {
                    servers.remove(self.id);
                }
                false
            }
        };
        update_list(&mut root, ALLOW, self.id, false);
        update_list(&mut root, DENY, self.id, deny);
        let updated = replace_object_preserving_trivia(source, root)
            .map_err(|_| WriteError::InvalidResource)?;
        if before.is_none() && updated == source {
            return Ok(None);
        }
        Ok(Some(updated))
    }
}

fn read_document(bytes: &[u8]) -> Result<Value, WriteError> {
    let root = object_value(bytes).map_err(|_| WriteError::InvalidResource)?;
    if root.get(SERVERS).is_some_and(|value| !value.is_object())
        || [ALLOW, DENY].iter().any(|key| root.get(key).is_some_and(|value| !value.is_array()))
    {
        return Err(WriteError::InvalidResource);
    }
    Ok(root)
}

fn update_list(root: &mut Value, key: &str, id: &str, include: bool) {
    if let Some(list) = root.get_mut(key).and_then(Value::as_array_mut) {
        list.retain(|value| value.as_str() != Some(id));
        if include {
            list.push(json!(id));
        }
    } else if include {
        root[key] = json!([id]);
    }
}

fn listed(root: &Value, key: &str, id: &str) -> bool {
    root[key].as_array().is_some_and(|list| list.iter().any(|value| value.as_str() == Some(id)))
}

fn validate_id(id: &str) -> Result<(), OmpMcpError> {
    if validate_new_name(id, &[]).is_ok_and(|name| name.0 == id) {
        Ok(())
    } else {
        Err(OmpMcpError::InvalidDefinition)
    }
}

fn import_entry(root: &Value, id: &str, value: &Value) -> Result<OmpMcpEntry, OmpMcpError> {
    validate_id(id)?;
    let object = value.as_object().ok_or(OmpMcpError::InvalidDefinition)?;
    if let Some(transport) = object.get("type").and_then(Value::as_str)
        && !matches!(transport, "stdio" | "http" | "streamable-http" | "sse")
    {
        return Err(OmpMcpError::UnsupportedTransport);
    }
    let fields = object
        .iter()
        .filter(|(key, _)| CONNECTION_FIELDS.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let connection =
        parse_native(Value::Object(fields)).map_err(|_| OmpMcpError::InvalidDefinition)?;
    let disabled = object.get("enabled").is_some_and(|value| {
        value.as_bool() == Some(false)
            || value.as_str().is_some_and(|text| text == "0" || text.eq_ignore_ascii_case("false"))
    });
    let enabled = !listed(root, DENY, id) && (!disabled || listed(root, ALLOW, id));
    Ok(OmpMcpEntry { connection, enabled })
}

fn project(connection: &McpConnectionDefinition) -> Result<Map<String, Value>, OmpMcpError> {
    validate_connection(connection).map_err(|_| OmpMcpError::InvalidDefinition)?;
    let mut fields = Map::new();
    match connection {
        McpConnectionDefinition::Stdio { command, args, env, cwd } => {
            fields.insert("type".into(), json!("stdio"));
            fields.insert("command".into(), json!(command));
            if !args.is_empty() {
                fields.insert("args".into(), json!(args));
            }
            if !env.is_empty() {
                fields.insert("env".into(), Value::Object(literals(env)?));
            }
            if let Some(cwd) = cwd {
                fields.insert("cwd".into(), json!(cwd));
            }
        }
        McpConnectionDefinition::StreamableHttp { url, headers }
        | McpConnectionDefinition::Sse { url, headers } => {
            let transport = if matches!(connection, McpConnectionDefinition::Sse { .. }) {
                "sse"
            } else {
                "http"
            };
            fields.insert("type".into(), json!(transport));
            fields.insert("url".into(), json!(url));
            if !headers.is_empty() {
                fields.insert("headers".into(), Value::Object(literals(headers)?));
            }
        }
    }
    Ok(fields)
}

fn literals(values: &BTreeMap<String, McpConfigValue>) -> Result<Map<String, Value>, OmpMcpError> {
    values
        .iter()
        .map(|(key, value)| match value {
            McpConfigValue::Literal { value } => Ok((key.clone(), json!(value))),
            McpConfigValue::Credential { .. } => Err(OmpMcpError::CredentialUnavailable),
        })
        .collect()
}
