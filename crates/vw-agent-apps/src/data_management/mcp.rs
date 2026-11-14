//! CC MCP normalization. Values remain host-private and never execute commands.
use super::source::{SourceCategory, SourceRecord};
use crate::mcp::{catalog::McpCatalogDraft, validation::parse_native};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use vw_api_types::{
    agent_apps::AppId,
    mcp_management::{McpCatalogRecord, McpCatalogScope, McpConnectionDefinition, McpFailureCode},
};

pub struct MigrationMcpRecord {
    pub record: McpCatalogRecord,
    pub connection: McpConnectionDefinition,
    pub applications: Vec<(AppId, bool)>,
}

/// Strictly decode every row before returning any prepared changes. Legacy
/// per-app JSON requires a separate source upgrade; it is never silently ignored.
pub fn normalize(
    records: &[&SourceRecord],
    scope: &McpCatalogScope,
) -> Result<Vec<MigrationMcpRecord>, McpFailureCode> {
    let mut rows = Vec::new();
    for record in records {
        if record.category != SourceCategory::Mcp {
            return Err(McpFailureCode::InvalidDefinition);
        }
        match record.origin.as_str() {
            "mcp_servers" => rows.push(record.fields.clone()),
            "/mcp" => {
                let root = record.fields.as_object().ok_or(McpFailureCode::InvalidDefinition)?;
                if root.keys().any(|key| key != "servers") {
                    return Err(McpFailureCode::InvalidDefinition);
                }
                let servers = root
                    .get("servers")
                    .and_then(Value::as_object)
                    .ok_or(McpFailureCode::InvalidDefinition)?;
                for (id, row) in servers {
                    if row.get("id").and_then(Value::as_str) != Some(id) {
                        return Err(McpFailureCode::InvalidDefinition);
                    }
                    rows.push(row.clone());
                }
            }
            _ => return Err(McpFailureCode::InvalidDefinition),
        }
    }
    let mut ids = BTreeSet::new();
    rows.into_iter()
        .map(|row| {
            let id = text(&row, "id")?.to_owned();
            if !ids.insert(id.clone()) {
                return Err(McpFailureCode::InvalidDefinition);
            }
            let sql = row.get("server_config").is_some();
            let native = if sql {
                decode(row.get("server_config").unwrap())?
            } else {
                row.get("server").cloned().ok_or(McpFailureCode::InvalidDefinition)?
            };
            // Paths and credentials cannot be transplanted between hosts. Executable
            // definitions are stored only; no capability or native deployment follows.
            let filtered = super::portable_policy::filter_portable_item(
                &native,
                super::portable_policy::MigrationLocation::CrossDevice,
            );
            if filtered.excluded.iter().any(|field| {
                !matches!(
                    field.reason,
                    super::portable_policy::ExclusionReason::ExecutionPermission
                )
            }) {
                return Err(McpFailureCode::InvalidDefinition);
            }
            if let Some(command) = native.get("command").and_then(Value::as_str) {
                if command.contains('/') || command.contains('\\') || command.starts_with('~') {
                    return Err(McpFailureCode::InvalidDefinition);
                }
            }
            let connection = parse_native(native).map_err(|_| McpFailureCode::InvalidDefinition)?;
            let tags = match row.get("tags") {
                None | Some(Value::Null) => Vec::new(),
                Some(value) => serde_json::from_value(decode(value)?)
                    .map_err(|_| McpFailureCode::InvalidDefinition)?,
            };
            let (record, connection) = McpCatalogDraft {
                id: id.clone(),
                name: Some(text(&row, "name")?.into()),
                description: optional(&row, "description")?,
                homepage: optional(&row, "homepage")?,
                documentation: optional(&row, "docs")?,
                tags,
                connection,
            }
            .into_record(scope.clone())?;
            if record.record.id.0 != id {
                return Err(McpFailureCode::InvalidDefinition);
            }
            let mut applications = BTreeMap::new();
            for (key, app) in [
                ("claude", AppId::Claude),
                ("codex", AppId::Codex),
                ("gemini", AppId::Gemini),
                ("grokbuild", AppId::GrokBuild),
                ("opencode", AppId::OpenCode),
                ("hermes", AppId::Hermes),
                ("mcode", AppId::Mcode),
                ("pi", AppId::Pi),
                ("agy", AppId::Agy),
                ("kimi", AppId::Kimi),
                ("omp", AppId::Omp),
                ("dsh", AppId::Dsh),
            ] {
                let value = if sql {
                    row.get(format!("enabled_{key}"))
                } else {
                    row.get("apps").and_then(|apps| apps.get(key))
                };
                let enabled = match value {
                    None => false,
                    Some(Value::Bool(v)) => *v,
                    Some(value) if sql && value.as_i64() == Some(0) => false,
                    Some(value) if sql && value.as_i64() == Some(1) => true,
                    _ => return Err(McpFailureCode::InvalidDefinition),
                };
                applications.insert(key, (app, enabled));
            }
            Ok(MigrationMcpRecord {
                record,
                connection,
                applications: applications.into_values().collect(),
            })
        })
        .collect()
}
fn text<'a>(row: &'a Value, key: &str) -> Result<&'a str, McpFailureCode> {
    row.get(key).and_then(Value::as_str).ok_or(McpFailureCode::InvalidDefinition)
}
fn optional(row: &Value, key: &str) -> Result<Option<String>, McpFailureCode> {
    match row.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        _ => Err(McpFailureCode::InvalidDefinition),
    }
}
fn decode(value: &Value) -> Result<Value, McpFailureCode> {
    match value {
        Value::String(value) => {
            serde_json::from_str(value).map_err(|_| McpFailureCode::InvalidDefinition)
        }
        value => Ok(value.clone()),
    }
}
