//! Codex MCP projection on the authorized execution host.

use crate::{
    mcp::{
        native_file::NativeFile,
        validation::{parse_native, validate_connection, validate_new_name},
    },
    providers::live::{
        commit::ProviderTransactions,
        patch_toml::TomlPatch,
        stage::FilePatch,
        write_guard::{AppWriteGuard, WriteError},
    },
};
use serde_json::Value;
use std::{collections::BTreeMap, ffi::OsStr};
use toml_edit::{Array, Item, Table, value};
use vw_api_types::{
    TenantScope,
    agent_apps::AppId,
    mcp_management::{McpConfigValue, McpConnectionDefinition},
};

const CONFIG: &str = "config.toml";
const SERVERS: &str = "mcp_servers";
const MANAGED_FIELDS: &[&str] =
    &["command", "args", "cwd", "env", "url", "http_headers", "headers", "type", "enabled"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexError {
    Write(WriteError),
    InvalidDefinition,
    UnsupportedTransport,
    UnsupportedField,
    UnresolvedCredential,
}

pub enum CodexEdit<'a> {
    /// Create a new binding. A same-name user entry conflicts.
    Insert(&'a McpConnectionDefinition),
    /// Enable and update an existing binding whose ownership the caller verified.
    Update(&'a McpConnectionDefinition),
    /// Disable or delete a binding whose ownership the caller verified.
    Remove,
}

/// Host-private native data that may contain credentials.
pub struct ImportedServer {
    pub enabled: bool,
    pub connection: McpConnectionDefinition,
}

/// Read each native entry independently. Canonical entries take precedence over legacy names.
pub fn read(
    guard: &mut AppWriteGuard,
) -> Result<BTreeMap<String, Result<ImportedServer, CodexError>>, CodexError> {
    let bytes = config_file(guard)?.read().map_err(CodexError::Write)?;
    let root = parse(bytes.as_deref()).map_err(CodexError::Write)?;
    let mut imported = BTreeMap::new();
    for servers in [legacy_servers(&root), server_table(&root, SERVERS)] {
        if let Some(servers) = servers.map_err(CodexError::Write)? {
            for (id, entry) in servers {
                imported.insert(id.clone(), import_entry(id, entry));
            }
        }
    }
    Ok(imported)
}

/// Project one binding under the provider transaction lock. The caller supplies an
/// authorized guard, host-resolved field values, and an ownership/revision preflight.
pub fn apply(
    guard: &mut AppWriteGuard,
    transactions: &ProviderTransactions,
    tenant: &TenantScope,
    id: &str,
    edit: CodexEdit<'_>,
    preflight: impl Fn() -> Result<(), WriteError>,
) -> Result<(), CodexError> {
    check_id(id)?;
    let file = config_file(guard)?;
    let edit = match edit {
        CodexEdit::Insert(connection) => ServerEdit::Insert(project(connection)?),
        CodexEdit::Update(connection) => ServerEdit::Update(project(connection)?),
        CodexEdit::Remove => ServerEdit::Remove,
    };
    let expected = file.read().map_err(CodexError::Write)?;
    file.commit(transactions, tenant, expected.as_deref(), &ServerPatch { id, edit }, preflight)
        .map_err(CodexError::Write)
}

fn config_file(guard: &mut AppWriteGuard) -> Result<NativeFile<'_>, CodexError> {
    if guard.app() != AppId::Codex {
        return Err(CodexError::Write(WriteError::InvalidResource));
    }
    Ok(NativeFile::new(guard, OsStr::new(CONFIG)))
}

fn check_id(id: &str) -> Result<(), CodexError> {
    let normalized = validate_new_name(id, &[]).map_err(|_| CodexError::InvalidDefinition)?;
    if normalized.0 != id {
        return Err(CodexError::InvalidDefinition);
    }
    Ok(())
}

fn parse(bytes: Option<&[u8]>) -> Result<toml::Table, WriteError> {
    let text =
        std::str::from_utf8(bytes.unwrap_or_default()).map_err(|_| WriteError::InvalidResource)?;
    toml::from_str(text).map_err(|_| WriteError::InvalidResource)
}

fn server_table<'a>(
    root: &'a toml::Table,
    name: &str,
) -> Result<Option<&'a toml::Table>, WriteError> {
    root.get(name).map(|item| item.as_table().ok_or(WriteError::InvalidResource)).transpose()
}

fn legacy_servers(root: &toml::Table) -> Result<Option<&toml::Table>, WriteError> {
    match root.get("mcp").and_then(toml::Value::as_table) {
        Some(mcp) => server_table(mcp, "servers"),
        None => Ok(None),
    }
}

fn import_entry(id: &str, entry: &toml::Value) -> Result<ImportedServer, CodexError> {
    check_id(id)?;
    let entry = entry.as_table().ok_or(CodexError::InvalidDefinition)?;
    if has_unmanaged_fields(entry) {
        return Err(CodexError::UnsupportedField);
    }
    let Value::Object(mut fields) =
        serde_json::to_value(entry).map_err(|_| CodexError::InvalidDefinition)?
    else {
        return Err(CodexError::InvalidDefinition);
    };
    let enabled = match fields.remove("enabled") {
        None => true,
        Some(Value::Bool(enabled)) => enabled,
        _ => return Err(CodexError::InvalidDefinition),
    };
    let connection =
        parse_native(Value::Object(fields)).map_err(|_| CodexError::InvalidDefinition)?;
    if matches!(connection, McpConnectionDefinition::Sse { .. }) {
        return Err(CodexError::UnsupportedTransport);
    }
    Ok(ImportedServer { enabled, connection })
}

fn project(connection: &McpConnectionDefinition) -> Result<Table, CodexError> {
    validate_connection(connection).map_err(|_| CodexError::InvalidDefinition)?;
    let mut server = Table::new();
    match connection {
        McpConnectionDefinition::Stdio { command, args, env, cwd } => {
            server.insert("command", value(command.as_str()));
            if !args.is_empty() {
                let args: Array = args.iter().map(String::as_str).collect();
                server.insert("args", value(args));
            }
            if let Some(cwd) = cwd {
                server.insert("cwd", value(cwd.as_str()));
            }
            if !env.is_empty() {
                server.insert("env", Item::Table(literal_fields(env)?));
            }
        }
        McpConnectionDefinition::StreamableHttp { url, headers } => {
            server.insert("url", value(url.as_str()));
            if !headers.is_empty() {
                server.insert("http_headers", Item::Table(literal_fields(headers)?));
            }
        }
        // Codex infers Streamable HTTP from a URL, so this format cannot encode SSE.
        McpConnectionDefinition::Sse { .. } => return Err(CodexError::UnsupportedTransport),
    }
    Ok(server)
}

fn literal_fields(fields: &BTreeMap<String, McpConfigValue>) -> Result<Table, CodexError> {
    let mut table = Table::new();
    for (name, field) in fields {
        let McpConfigValue::Literal { value: text } = field else {
            return Err(CodexError::UnresolvedCredential);
        };
        table.insert(name, value(text.as_str()));
    }
    Ok(table)
}

enum ServerEdit {
    Insert(Table),
    Update(Table),
    Remove,
}

struct ServerPatch<'a> {
    id: &'a str,
    edit: ServerEdit,
}

impl FilePatch for ServerPatch<'_> {
    fn apply(&self, before: Option<&[u8]>) -> Result<Option<Vec<u8>>, WriteError> {
        let root = parse(before)?;
        let current = server_table(&root, SERVERS)?.and_then(|servers| servers.get(self.id));
        // A legacy entry is an import source. Never claim or edit it implicitly.
        if legacy_servers(&root)?.is_some_and(|servers| servers.contains_key(self.id)) {
            return Err(WriteError::Conflict);
        }
        let path = vec![SERVERS.to_owned(), self.id.to_owned()];
        let patch = match &self.edit {
            ServerEdit::Insert(server) => {
                if current.is_some() {
                    return Err(WriteError::Conflict);
                }
                TomlPatch { set: vec![(path, Item::Table(server.clone()))], ..TomlPatch::default() }
            }
            ServerEdit::Update(server) => {
                let current = current
                    .ok_or(WriteError::Conflict)?
                    .as_table()
                    .ok_or(WriteError::InvalidResource)?;
                // A connection change must not carry over settings such as dynamic credentials.
                if has_unmanaged_fields(current) {
                    return Err(WriteError::InvalidResource);
                }
                update_patch(&path, server)
            }
            ServerEdit::Remove => {
                let Some(current) = current else {
                    return Ok(before.map(<[u8]>::to_vec));
                };
                current.as_table().ok_or(WriteError::InvalidResource)?;
                TomlPatch { remove: vec![path], ..TomlPatch::default() }
            }
        };
        patch.apply(before).map(Some).map_err(|_| WriteError::InvalidResource)
    }
}

fn update_patch(path: &[String], server: &Table) -> TomlPatch {
    let field_path = |field: &str| {
        let mut path = path.to_vec();
        path.push(field.to_owned());
        path
    };
    TomlPatch {
        set: server.iter().map(|(key, item)| (field_path(key), item.clone())).collect(),
        remove: MANAGED_FIELDS
            .iter()
            .filter(|key| !server.contains_key(key))
            .map(|key| field_path(key))
            .collect(),
        ..TomlPatch::default()
    }
}

fn has_unmanaged_fields(table: &toml::Table) -> bool {
    table.keys().any(|key| !MANAGED_FIELDS.contains(&key.as_str()))
}
