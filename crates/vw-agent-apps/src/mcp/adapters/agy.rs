//! AGY MCP configuration on the authorized execution host.

use crate::{
    mcp::{
        native_file::{JsonServerEdit, JsonServerPatch, NativeFile},
        validation::{parse_native, validate_connection, validate_new_name},
    },
    providers::{
        live::{
            commit::ProviderTransactions,
            patch_json::object_value,
            stage::{FilePatch, WholeFile},
            write_guard::{AppWriteGuard, WriteError, lock_directory},
        },
        paths::open_beneath,
    },
};
use serde_json::{Map, Value, json};
use std::{collections::BTreeMap, ffi::OsStr, path::Path};
use vw_api_types::{
    TenantScope,
    agent_apps::AppId,
    app_providers::paths::ConfigPathState,
    mcp_management::{McpConfigValue, McpConnectionDefinition},
};

const SERVERS: &str = "mcpServers";
const CONNECTION_FIELDS: &[&str] = &[
    "type",
    "transport",
    "command",
    "args",
    "env",
    "cwd",
    "url",
    "serverUrl",
    "headers",
    "http_headers",
];

#[derive(Debug, PartialEq, Eq)]
pub enum AgyMcpError {
    UnsupportedTransport,
    InvalidDefinition,
    CredentialUnavailable,
    File(WriteError),
}

impl From<WriteError> for AgyMcpError {
    fn from(error: WriteError) -> Self {
        Self::File(error)
    }
}

pub enum AgyMcpEdit<'a> {
    /// Publish a new binding. An existing same-name service conflicts.
    Insert(&'a McpConnectionDefinition),
    /// Replace connection fields and enable a verified binding.
    Update(&'a McpConnectionDefinition),
    /// Keep a verified binding and its options in the native file.
    Disable,
    Remove,
}

pub struct AgyNativeServer {
    pub connection: McpConnectionDefinition,
    pub enabled: bool,
}

pub struct AgyMcp(AppWriteGuard);

impl AgyMcp {
    /// The executor supplies the authorized installation's host home.
    /// AGY shares this configuration between native and provisioned profiles.
    pub fn open(home: &Path) -> Result<Self, WriteError> {
        Self::open_existing(home)?.ok_or(WriteError::InvalidResource)
    }

    pub(in crate::mcp) fn open_existing(home: &Path) -> Result<Option<Self>, WriteError> {
        if !home.is_absolute() {
            return Err(WriteError::InvalidResource);
        }
        match open_beneath(home, &home.join(".gemini/config"), true) {
            Ok(directory) => Ok(Some(Self(lock_directory(AppId::Agy, directory)?))),
            Err(ConfigPathState::Absent) => Ok(None),
            Err(_) => Err(WriteError::InvalidResource),
        }
    }

    fn file(&mut self) -> NativeFile<'_> {
        NativeFile::new(&mut self.0, OsStr::new("mcp_config.json"))
    }

    /// Keep these candidates on the host until catalog credential policy has been applied.
    pub fn read(
        &mut self,
    ) -> Result<BTreeMap<String, Result<AgyNativeServer, AgyMcpError>>, WriteError> {
        let bytes = self.file().read()?;
        Ok(server_map(nonblank(bytes.as_deref()))?
            .into_iter()
            .map(|(id, value)| {
                let server = validate_id(&id).and_then(|()| import_server(value));
                (id, server)
            })
            .collect())
    }

    /// Supply host-private definitions with resolved credential values. Recheck the
    /// saved revision and ownership before publishing updates, disables or removals.
    pub fn apply(
        &mut self,
        transactions: &ProviderTransactions,
        tenant: &TenantScope,
        id: &str,
        edit: AgyMcpEdit<'_>,
        preflight: impl Fn() -> Result<(), WriteError>,
    ) -> Result<(), AgyMcpError> {
        validate_id(id)?;
        let file = self.file();
        let expected = file.read()?;
        let before = expected.as_deref();
        let config = nonblank(before);
        let fields;
        let remove;
        let edit = match edit {
            AgyMcpEdit::Insert(connection) => {
                fields = project(connection)?;
                JsonServerEdit::Insert(&fields)
            }
            AgyMcpEdit::Update(connection) => {
                fields = project(connection)?;
                remove = CONNECTION_FIELDS
                    .iter()
                    .copied()
                    .filter(|key| !fields.contains_key(*key))
                    .chain(["enabled"])
                    .collect::<Vec<_>>();
                JsonServerEdit::Update { set: &fields, remove: &remove }
            }
            AgyMcpEdit::Disable if server_map(config)?.contains_key(id) => {
                fields = Map::from_iter([("disabled".into(), Value::Bool(true))]);
                JsonServerEdit::Update { set: &fields, remove: &[] }
            }
            AgyMcpEdit::Disable | AgyMcpEdit::Remove => JsonServerEdit::Remove,
        };
        let patch = JsonServerPatch { section: SERVERS, id, edit };
        // A no-op on AGY's initial blank file must keep the original bytes.
        let after = patch.apply(config)?.or_else(|| before.map(<[u8]>::to_vec));
        file.commit(transactions, tenant, before, &WholeFile::new(before, after), preflight)?;
        Ok(())
    }
}

fn nonblank(bytes: Option<&[u8]>) -> Option<&[u8]> {
    bytes.filter(|bytes| !bytes.iter().all(u8::is_ascii_whitespace))
}

fn server_map(config: Option<&[u8]>) -> Result<Map<String, Value>, WriteError> {
    let mut root =
        object_value(config.unwrap_or(b"{}")).map_err(|_| WriteError::InvalidResource)?;
    match root.as_object_mut().and_then(|root| root.remove(SERVERS)) {
        Some(Value::Object(servers)) => Ok(servers),
        None => Ok(Map::new()),
        Some(_) => Err(WriteError::InvalidResource),
    }
}

fn validate_id(id: &str) -> Result<(), AgyMcpError> {
    if validate_new_name(id, &[]).is_ok_and(|name| name.0 == id) {
        Ok(())
    } else {
        Err(AgyMcpError::InvalidDefinition)
    }
}

fn import_server(value: Value) -> Result<AgyNativeServer, AgyMcpError> {
    let Value::Object(mut fields) = value else {
        return Err(AgyMcpError::InvalidDefinition);
    };
    if fields.get("type").and_then(Value::as_str) == Some("sse") {
        return Err(AgyMcpError::UnsupportedTransport);
    }
    let enabled = match fields.remove("disabled") {
        None => true,
        Some(Value::Bool(disabled)) => !disabled,
        Some(_) => return Err(AgyMcpError::InvalidDefinition),
    };
    // Native options remain in the file. Only connection fields go into the catalog.
    fields.retain(|key, _| CONNECTION_FIELDS.contains(&key.as_str()));
    if let Some(url) = fields.remove("serverUrl")
        && fields.insert("url".into(), url).is_some()
    {
        return Err(AgyMcpError::InvalidDefinition);
    }
    let connection =
        parse_native(Value::Object(fields)).map_err(|_| AgyMcpError::InvalidDefinition)?;
    Ok(AgyNativeServer { connection, enabled })
}

fn project(connection: &McpConnectionDefinition) -> Result<Map<String, Value>, AgyMcpError> {
    let mut fields = Map::from_iter([("disabled".into(), Value::Bool(false))]);
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
        McpConnectionDefinition::StreamableHttp { url, headers } => {
            fields.insert("serverUrl".into(), json!(url));
            if !headers.is_empty() {
                fields.insert("headers".into(), Value::Object(literal_fields(headers)?));
            }
        }
        McpConnectionDefinition::Sse { .. } => return Err(AgyMcpError::UnsupportedTransport),
    }
    validate_connection(connection).map_err(|_| AgyMcpError::InvalidDefinition)?;
    Ok(fields)
}

fn literal_fields(
    fields: &BTreeMap<String, McpConfigValue>,
) -> Result<Map<String, Value>, AgyMcpError> {
    fields
        .iter()
        .map(|(key, value)| match value {
            McpConfigValue::Literal { value } => Ok((key.clone(), json!(value))),
            McpConfigValue::Credential { .. } => Err(AgyMcpError::CredentialUnavailable),
        })
        .collect()
}
