//! Grok Build MCP projection for the authorized host's .grok/config.toml.
use crate::{
    mcp::{
        native_file::NativeFile,
        validation::{parse_native, validate_connection, validate_new_name},
    },
    providers::live::{
        commit::ProviderTransactions,
        patch_toml::TomlPatch,
        stage::WholeFile,
        write_guard::{AppWriteGuard, WriteError},
    },
};
use std::{collections::BTreeMap, ffi::OsStr};
use toml_edit::{Array, DocumentMut, Item, Table, value};
use vw_api_types::{
    TenantScope,
    agent_apps::AppId,
    mcp_management::{McpConfigValue, McpConnectionDefinition},
};

const SERVERS: &str = "mcp_servers";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrokMcpError {
    Native(WriteError),
    InvalidDefinition,
    UnresolvedCredential,
    UnsupportedTransport,
}

impl From<WriteError> for GrokMcpError {
    fn from(error: WriteError) -> Self {
        Self::Native(error)
    }
}

pub enum GrokMcpEdit<'a> {
    Insert(&'a McpConnectionDefinition),
    /// The executor must verify ownership of this binding.
    Update(&'a McpConnectionDefinition),
    /// Disable or delete a verified binding. An absent entry is already revoked.
    Remove,
}

fn native_file(guard: &mut AppWriteGuard) -> Result<NativeFile<'_>, WriteError> {
    if guard.app() != AppId::GrokBuild {
        return Err(WriteError::InvalidResource);
    }
    Ok(NativeFile::new(guard, OsStr::new("config.toml")))
}

/// Read import candidates on the host. Definitions can contain credentials and
/// must pass catalog import policy before storage or public projection.
pub fn read(
    guard: &mut AppWriteGuard,
) -> Result<BTreeMap<String, Result<McpConnectionDefinition, GrokMcpError>>, GrokMcpError> {
    let bytes = native_file(guard)?.read()?;
    let text = std::str::from_utf8(bytes.as_deref().unwrap_or_default())
        .map_err(|_| WriteError::InvalidResource)?;
    let document: toml::Table = text.parse().map_err(|_| WriteError::InvalidResource)?;
    let Some(servers) = document.get(SERVERS) else {
        return Ok(BTreeMap::new());
    };
    let servers = servers.as_table().ok_or(WriteError::InvalidResource)?;
    Ok(servers.iter().map(|(id, entry)| (id.clone(), import_entry(id, entry))).collect())
}

fn import_entry(id: &str, entry: &toml::Value) -> Result<McpConnectionDefinition, GrokMcpError> {
    validate_id(id)?;
    let native = serde_json::to_value(entry).map_err(|_| GrokMcpError::InvalidDefinition)?;
    let connection = parse_native(native).map_err(|_| GrokMcpError::InvalidDefinition)?;
    if matches!(connection, McpConnectionDefinition::Sse { .. }) {
        return Err(GrokMcpError::UnsupportedTransport);
    }
    Ok(connection)
}

/// Publish one edit through the supplier transaction guard. Definitions are
/// host-private; env and header values must already have been resolved.
pub fn apply(
    guard: &mut AppWriteGuard,
    transactions: &ProviderTransactions,
    tenant: &TenantScope,
    id: &str,
    edit: GrokMcpEdit<'_>,
    preflight: impl Fn() -> Result<(), WriteError>,
) -> Result<(), GrokMcpError> {
    validate_id(id)?;
    let file = native_file(guard)?;
    let expected = file.read()?;
    let before = expected.as_deref();
    let text =
        std::str::from_utf8(before.unwrap_or_default()).map_err(|_| WriteError::InvalidResource)?;
    let doc: DocumentMut = text.parse().map_err(|_| WriteError::InvalidResource)?;
    let existing = match doc.get(SERVERS) {
        Some(section) => section.as_table_like().ok_or(WriteError::InvalidResource)?.get(id),
        None => None,
    };

    let path = vec![SERVERS.into(), id.into()];
    let mut patch = TomlPatch::default();
    match edit {
        GrokMcpEdit::Insert(_) if existing.is_some() => return Err(WriteError::Conflict.into()),
        GrokMcpEdit::Update(_) if existing.is_none() => return Err(WriteError::Conflict.into()),
        GrokMcpEdit::Update(_) | GrokMcpEdit::Remove
            if existing.is_some_and(|entry| entry.as_table_like().is_none()) =>
        {
            return Err(WriteError::InvalidResource.into());
        }
        GrokMcpEdit::Insert(connection) | GrokMcpEdit::Update(connection) => {
            patch.set.push((path, Item::Table(project(connection)?)));
        }
        GrokMcpEdit::Remove => patch.remove.push(path),
    }

    let after = if matches!(edit, GrokMcpEdit::Remove) && existing.is_none() {
        before.map(<[u8]>::to_vec)
    } else {
        Some(patch.apply(before).map_err(|_| WriteError::InvalidResource)?)
    };
    file.commit(transactions, tenant, before, &WholeFile::new(before, after), preflight)?;
    Ok(())
}

fn validate_id(id: &str) -> Result<(), GrokMcpError> {
    if validate_new_name(id, &[]).is_ok_and(|validated| validated.0 == id) {
        Ok(())
    } else {
        Err(GrokMcpError::InvalidDefinition)
    }
}

fn project(connection: &McpConnectionDefinition) -> Result<Table, GrokMcpError> {
    validate_connection(connection).map_err(|_| GrokMcpError::InvalidDefinition)?;
    let mut table = Table::new();
    match connection {
        McpConnectionDefinition::Stdio { command, args, env, cwd } => {
            table.insert("command", value(command.as_str()));
            if !args.is_empty() {
                table.insert("args", value(args.iter().map(String::as_str).collect::<Array>()));
            }
            if !env.is_empty() {
                table.insert("env", Item::Table(literal_fields(env)?));
            }
            if let Some(cwd) = cwd {
                table.insert("cwd", value(cwd.as_str()));
            }
        }
        McpConnectionDefinition::StreamableHttp { url, headers } => {
            table.insert("url", value(url.as_str()));
            if !headers.is_empty() {
                table.insert("headers", Item::Table(literal_fields(headers)?));
            }
        }
        // The pinned source writes both remote transports as url + headers.
        // Reading that shape can only recover StreamableHttp.
        McpConnectionDefinition::Sse { .. } => return Err(GrokMcpError::UnsupportedTransport),
    }
    Ok(table)
}

fn literal_fields(fields: &BTreeMap<String, McpConfigValue>) -> Result<Table, GrokMcpError> {
    let mut table = Table::new();
    for (key, field) in fields {
        let McpConfigValue::Literal { value: text } = field else {
            return Err(GrokMcpError::UnresolvedCredential);
        };
        table.insert(key, value(text.as_str()));
    }
    Ok(table)
}
