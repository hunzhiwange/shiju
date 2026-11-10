//! Gemini settings projection on an authorized execution host.

use crate::{
    mcp::{
        native_file::{JsonServerEdit, JsonServerPatch, NativeFile},
        validation::{parse_native, validate_connection, validate_new_name},
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
    mcp_management::{McpConfigValue, McpConnectionDefinition, McpTransport},
};

const SERVERS: &str = "mcpServers";
const CONNECTION_FIELDS: &[&str] = &[
    "type",
    "transport",
    "command",
    "args",
    "env",
    "cwd",
    "httpUrl",
    "url",
    "headers",
    "http_headers",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeminiMcpError {
    UnsupportedTransport,
    InvalidDefinition,
    CredentialUnavailable,
    File(WriteError),
}

impl From<WriteError> for GeminiMcpError {
    fn from(error: WriteError) -> Self {
        Self::File(error)
    }
}

pub enum GeminiMcpEdit<'a> {
    /// Publish a new binding. An existing same-name service conflicts.
    Insert(&'a McpConnectionDefinition),
    /// Change a service whose ownership the executor has verified.
    Update(&'a McpConnectionDefinition),
    /// Disable or delete a service whose ownership the executor has verified.
    Remove,
}

pub struct GeminiMcp<'a>(NativeFile<'a>);

impl<'a> GeminiMcp<'a> {
    /// The guard must cover the authorized installation's Gemini settings directory.
    pub fn new(guard: &'a mut AppWriteGuard) -> Result<Self, WriteError> {
        if guard.app() != AppId::Gemini {
            return Err(WriteError::InvalidResource);
        }
        Ok(Self(NativeFile::new(guard, OsStr::new("settings.json"))))
    }

    /// Return host-private import candidates. Env, headers and URLs may contain secrets.
    /// Validate catalog credential policy before persisting or exposing any candidate.
    pub fn read(
        &self,
    ) -> Result<BTreeMap<String, Result<McpConnectionDefinition, GeminiMcpError>>, WriteError> {
        let Some(bytes) = self.0.read()? else {
            return Ok(BTreeMap::new());
        };
        let mut root = object_value(&bytes).map_err(|_| WriteError::InvalidResource)?;
        match root.as_object_mut().and_then(|root| root.remove(SERVERS)) {
            None => Ok(BTreeMap::new()),
            Some(Value::Object(servers)) => {
                Ok(servers.into_iter().map(|(id, value)| (id, import_connection(value))).collect())
            }
            Some(_) => Err(WriteError::InvalidResource),
        }
    }

    /// The executor supplies the host-private definition with credential references
    /// resolved into literal env/header values. Recheck ownership and revision in preflight.
    pub fn apply(
        self,
        transactions: &ProviderTransactions,
        tenant: &TenantScope,
        id: &str,
        edit: GeminiMcpEdit<'_>,
        preflight: impl Fn() -> Result<(), WriteError>,
    ) -> Result<(), GeminiMcpError> {
        if !validate_new_name(id, &[]).is_ok_and(|name| name.0 == id) {
            return Err(GeminiMcpError::InvalidDefinition);
        }
        let fields;
        let remove;
        let edit = match edit {
            GeminiMcpEdit::Insert(connection) => {
                fields = project_connection(connection)?;
                JsonServerEdit::Insert(&fields)
            }
            GeminiMcpEdit::Update(connection) => {
                fields = project_connection(connection)?;
                remove = CONNECTION_FIELDS
                    .iter()
                    .copied()
                    .filter(|key| !fields.contains_key(*key))
                    .collect::<Vec<_>>();
                JsonServerEdit::Update { set: &fields, remove: &remove }
            }
            GeminiMcpEdit::Remove => JsonServerEdit::Remove,
        };
        let expected = self.0.read()?;
        let patch = JsonServerPatch { section: SERVERS, id, edit };
        self.0.commit(transactions, tenant, expected.as_deref(), &patch, preflight)?;
        Ok(())
    }
}

fn project_connection(
    connection: &McpConnectionDefinition,
) -> Result<Map<String, Value>, GeminiMcpError> {
    validate_connection(connection).map_err(|_| GeminiMcpError::InvalidDefinition)?;
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
            let url_field = match connection {
                McpConnectionDefinition::StreamableHttp { .. } => "httpUrl",
                _ => "url",
            };
            fields.insert(url_field.into(), json!(url));
            if !headers.is_empty() {
                fields.insert("headers".into(), Value::Object(literal_fields(headers)?));
            }
        }
    }
    Ok(fields)
}

fn literal_fields(
    values: &BTreeMap<String, McpConfigValue>,
) -> Result<Map<String, Value>, GeminiMcpError> {
    values
        .iter()
        .map(|(name, value)| match value {
            McpConfigValue::Literal { value } => Ok((name.clone(), json!(value))),
            McpConfigValue::Credential { .. } => Err(GeminiMcpError::CredentialUnavailable),
        })
        .collect()
}

fn import_connection(value: Value) -> Result<McpConnectionDefinition, GeminiMcpError> {
    let Value::Object(mut fields) = value else {
        return Err(GeminiMcpError::InvalidDefinition);
    };
    let declared = match fields.remove("type") {
        None => None,
        Some(Value::String(value)) => Some(match value.as_str() {
            "stdio" => McpTransport::Stdio,
            "http" | "streamable_http" | "streamable-http" | "streamableHttp" => {
                McpTransport::StreamableHttp
            }
            "sse" => McpTransport::Sse,
            _ => return Err(GeminiMcpError::UnsupportedTransport),
        }),
        Some(_) => return Err(GeminiMcpError::InvalidDefinition),
    };
    let (transport, native_type) = match (
        fields.contains_key("command"),
        fields.contains_key("httpUrl"),
        fields.contains_key("url"),
    ) {
        (true, false, false) => (McpTransport::Stdio, "stdio"),
        (false, true, false) => (McpTransport::StreamableHttp, "http"),
        (false, false, true) => (McpTransport::Sse, "sse"),
        _ => return Err(GeminiMcpError::InvalidDefinition),
    };
    if declared.is_some_and(|declared| declared != transport) {
        return Err(GeminiMcpError::InvalidDefinition);
    }
    for field in ["timeout", "trust", "description", "includeTools", "excludeTools"] {
        fields.remove(field);
    }
    if let Some(http_url) = fields.remove("httpUrl") {
        fields.insert("url".into(), http_url);
    }
    fields.insert("type".into(), json!(native_type));
    parse_native(Value::Object(fields)).map_err(|_| GeminiMcpError::InvalidDefinition)
}
