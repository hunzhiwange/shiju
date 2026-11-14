//! Preserve legacy v2 source sections without invoking CC's migration code.
use serde_json::Value;
use super::source::{MAX_JSON_VERSION, SourceCategory, SourceError, SourceRecord, push_record};

pub(super) fn read(bytes: &[u8], records: &mut Vec<SourceRecord>) -> Result<u32, SourceError> {
    let first = bytes.iter().copied().find(|byte| !byte.is_ascii_whitespace());
    if first != Some(b'{') { return Err(SourceError::UnknownFormat); }
    let value: Value = serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidData)?;
    let root = value.as_object().ok_or(SourceError::UnknownFormat)?;
    // MultiAppConfig defaults an omitted version to 2. Do not confuse this JSON
    // format version with SQLite's user_version or the application release.
    let version = match root.get("version") {
        Some(value) => value.as_u64().and_then(|v| u32::try_from(v).ok()).ok_or(SourceError::InvalidData)?,
        None => MAX_JSON_VERSION,
    };
    if version > MAX_JSON_VERSION {
        return Err(SourceError::FutureVersion { found: version, supported: MAX_JSON_VERSION });
    }
    if version != MAX_JSON_VERSION { return Err(SourceError::UnsupportedVersion); }
    for (key, fields) in root {
        let category = match key.as_str() {
            "version" => continue,
            "mcp" => { require_object(fields)?; SourceCategory::Mcp },
            "prompts" => { require_object(fields)?; SourceCategory::Rules },
            "skills" => { require_object(fields)?; SourceCategory::Skills },
            "common_config_snippets" => { require_object(fields)?; SourceCategory::Preferences },
            "claude_common_config_snippet" => {
                if !fields.is_null() && !fields.is_string() { return Err(SourceError::InvalidData); }
                SourceCategory::Preferences
            }
            _ => {
                // Apps are flattened in MultiAppConfig, not nested under apps.
                // Retain the whole manager including current and unknown fields;
                // AppId and per-provider validation belong to domain preview.
                let manager = fields.as_object().ok_or(SourceError::UnknownFormat)?;
                if !manager.get("providers").is_some_and(Value::is_object)
                    || !manager.get("current").is_some_and(Value::is_string) {
                    return Err(SourceError::UnknownFormat);
                }
                SourceCategory::Providers
            }
        };
        push_record(records, SourceRecord {
            category,
            origin: format!("/{}", key.replace('~', "~0").replace('/', "~1")),
            fields: fields.clone(),
        })?;
    }
    if records.is_empty() { return Err(SourceError::UnknownFormat); }
    Ok(version)
}
fn require_object(value: &Value) -> Result<(), SourceError> {
    if !value.is_object() { return Err(SourceError::InvalidData); }
    Ok(())
}
