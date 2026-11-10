//! Hermes config.yaml projection for the authorized execution host.

use crate::{
    mcp::{
        native_file::NativeFile,
        validation::{parse_native, validate_connection, validate_new_name},
    },
    providers::live::{
        patch_yaml::patch_mcp_server,
        stage::FilePatch,
        write_guard::{AppWriteGuard, WriteError},
    },
};
use serde_json::{Map, Value};
use serde_yaml::{Mapping, Value as Yaml};
use std::{collections::BTreeMap, ffi::OsStr};
use vw_api_types::{
    agent_apps::AppId,
    mcp_management::{McpConfigValue, McpConnectionDefinition},
};

const SERVER_SECTION: &str = "mcp_servers";
const CONNECTION_FIELDS: [&str; 6] = ["command", "args", "env", "url", "headers", "transport"];

/// The executor resolves and locks the authorized Hermes directory through the provider path layer.
pub fn file(guard: &mut AppWriteGuard) -> Result<NativeFile<'_>, WriteError> {
    if guard.app() != AppId::Hermes {
        return Err(WriteError::InvalidResource);
    }
    Ok(NativeFile::new(guard, OsStr::new("config.yaml")))
}

#[derive(Debug, PartialEq, Eq)]
pub enum HermesError {
    InvalidConfig,
    InvalidDefinition,
    UnsupportedTransport,
    UnsupportedField(&'static str),
    UnresolvedCredential,
}

pub struct NativeServer {
    pub connection: McpConnectionDefinition,
    pub enabled: bool,
}

pub type NativeServers = BTreeMap<String, Result<NativeServer, HermesError>>;

/// Keep the imported connection on the host. The catalog import owner must vet
/// sensitive values before including anything in previews or managed storage.
pub fn read_servers(snapshot: Option<&[u8]>) -> Result<NativeServers, HermesError> {
    server_map(snapshot)?
        .into_iter()
        .map(|(id, value)| {
            let id = id.as_str().ok_or(HermesError::InvalidConfig)?;
            let server = validate_id(id).and_then(|()| read_server(&value));
            Ok((id.to_owned(), server))
        })
        .collect()
}

fn server_map(snapshot: Option<&[u8]>) -> Result<Mapping, HermesError> {
    let mut document: Yaml = serde_yaml::from_slice(snapshot.unwrap_or_default())
        .map_err(|_| HermesError::InvalidConfig)?;
    let section = match &mut document {
        Yaml::Null => None,
        Yaml::Mapping(root) => root.remove(SERVER_SECTION),
        _ => return Err(HermesError::InvalidConfig),
    };
    match section {
        None | Some(Yaml::Null) => Ok(Mapping::new()),
        Some(Yaml::Mapping(servers)) => Ok(servers),
        _ => Err(HermesError::InvalidConfig),
    }
}

fn validate_id(id: &str) -> Result<(), HermesError> {
    let validated = validate_new_name(id, &[]).map_err(|_| HermesError::InvalidDefinition)?;
    if id != validated.0 {
        return Err(HermesError::InvalidDefinition);
    }
    Ok(())
}

fn read_server(value: &Yaml) -> Result<NativeServer, HermesError> {
    let server = value.as_mapping().ok_or(HermesError::InvalidDefinition)?;
    let enabled = match server.get("enabled") {
        None => true,
        Some(value) => value.as_bool().ok_or(HermesError::InvalidDefinition)?,
    };
    let (transport, fields): (_, &[_]) =
        match (server.contains_key("command"), server.contains_key("url"), server.get("transport"))
        {
            (true, false, None) => ("stdio", &["command", "args", "env"]),
            (false, true, None) => ("http", &["url", "headers"]),
            (false, true, Some(value)) if value.as_str() == Some("http") => {
                ("http", &["url", "headers"])
            }
            (false, true, Some(value)) if value.as_str() == Some("sse") => {
                ("sse", &["url", "headers"])
            }
            (true, false, Some(_)) | (false, true, Some(_)) => {
                return Err(HermesError::UnsupportedTransport);
            }
            _ => return Err(HermesError::InvalidDefinition),
        };
    let mut definition = Map::new();
    definition.insert("type".into(), Value::from(transport));
    for &field in fields {
        if let Some(value) = server.get(field) {
            let value = serde_json::to_value(value).map_err(|_| HermesError::InvalidDefinition)?;
            definition.insert(field.into(), value);
        }
    }
    let connection =
        parse_native(Value::Object(definition)).map_err(|_| HermesError::InvalidDefinition)?;
    Ok(NativeServer { connection, enabled })
}

pub enum ServerEdit<'a> {
    Insert(&'a McpConnectionDefinition),
    /// Replace connection fields while keeping the current enabled state.
    Update(&'a McpConnectionDefinition),
    Disable,
    Remove,
}

enum PatchEdit {
    Insert(Mapping),
    Update(Mapping),
    Disable,
    Remove,
}

/// A patch of one binding. The executor must check ownership and revision in
/// NativeFile's preflight before using Update, Disable or Remove.
pub struct ServerPatch {
    id: String,
    edit: PatchEdit,
}

impl ServerPatch {
    /// Use a host-private definition whose credential references have already been resolved.
    pub fn new(id: &str, edit: ServerEdit<'_>) -> Result<Self, HermesError> {
        validate_id(id)?;
        let edit = match edit {
            ServerEdit::Insert(connection) => PatchEdit::Insert(project(connection)?),
            ServerEdit::Update(connection) => PatchEdit::Update(project(connection)?),
            ServerEdit::Disable => PatchEdit::Disable,
            ServerEdit::Remove => PatchEdit::Remove,
        };
        Ok(Self { id: id.into(), edit })
    }
}

fn project(connection: &McpConnectionDefinition) -> Result<Mapping, HermesError> {
    validate_connection(connection).map_err(|_| HermesError::InvalidDefinition)?;
    let mut fields = Mapping::new();
    match connection {
        McpConnectionDefinition::Stdio { command, args, env, cwd } => {
            if cwd.is_some() {
                return Err(HermesError::UnsupportedField("cwd"));
            }
            fields.insert("command".into(), Yaml::from(command.as_str()));
            if !args.is_empty() {
                fields.insert(
                    "args".into(),
                    serde_yaml::to_value(args).map_err(|_| HermesError::InvalidDefinition)?,
                );
            }
            if !env.is_empty() {
                fields.insert("env".into(), literals(env)?);
            }
        }
        McpConnectionDefinition::StreamableHttp { url, headers }
        | McpConnectionDefinition::Sse { url, headers } => {
            fields.insert("url".into(), Yaml::from(url.as_str()));
            if !headers.is_empty() {
                fields.insert("headers".into(), literals(headers)?);
            }
            if matches!(connection, McpConnectionDefinition::Sse { .. }) {
                fields.insert("transport".into(), "sse".into());
            }
        }
    }
    Ok(fields)
}

fn literals(values: &BTreeMap<String, McpConfigValue>) -> Result<Yaml, HermesError> {
    let values = values
        .iter()
        .map(|(key, value)| match value {
            McpConfigValue::Literal { value } => {
                Ok((Yaml::from(key.as_str()), Yaml::from(value.as_str())))
            }
            McpConfigValue::Credential { .. } => Err(HermesError::UnresolvedCredential),
        })
        .collect::<Result<Mapping, _>>()?;
    Ok(Yaml::Mapping(values))
}

impl FilePatch for ServerPatch {
    fn apply(&self, before: Option<&[u8]>) -> Result<Option<Vec<u8>>, WriteError> {
        let mut servers = server_map(before).map_err(|_| WriteError::InvalidResource)?;
        let current = servers.remove(self.id.as_str());
        let replacement = match &self.edit {
            PatchEdit::Insert(fields) => {
                if current.is_some() {
                    return Err(WriteError::Conflict);
                }
                let mut fields = fields.clone();
                fields.insert("enabled".into(), true.into());
                Some(fields)
            }
            PatchEdit::Update(fields) => {
                let mut current = server_fields(current.ok_or(WriteError::Conflict)?)?;
                for field in CONNECTION_FIELDS {
                    current.remove(field);
                }
                current.extend(fields.clone());
                Some(current)
            }
            PatchEdit::Disable => {
                let Some(current) = current else {
                    return Ok(before.map(<[u8]>::to_vec));
                };
                let mut current = server_fields(current)?;
                if current.get("enabled").and_then(Yaml::as_bool) == Some(false) {
                    return Ok(before.map(<[u8]>::to_vec));
                }
                current.insert("enabled".into(), false.into());
                Some(current)
            }
            PatchEdit::Remove => {
                if current.is_none() {
                    return Ok(before.map(<[u8]>::to_vec));
                }
                None
            }
        };
        patch_mcp_server(before, &self.id, replacement)
            .map(Some)
            .map_err(|_| WriteError::InvalidResource)
    }
}

fn server_fields(value: Yaml) -> Result<Mapping, WriteError> {
    match value {
        Yaml::Mapping(fields) => Ok(fields),
        _ => Err(WriteError::InvalidResource),
    }
}
