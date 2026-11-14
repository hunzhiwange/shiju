//! Single-file Claude Code usage projection. No discovery, scheduling or ledger writes.
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

use serde_json::Value;
use vw_api_types::usage::{UsageProvider, UsageUnixSeconds};

use super::tokens::{ParsedUsage, UsageProtocol, parse_response};

/// Internal projection: project paths must not be exposed in public usage DTOs.
/// Callers bind host, scope, instance and stable file identity before saving.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeUsageMessage {
    pub message_id: String,
    pub session_id: Option<String>,
    /// Native cwd, matching the session reader; never guessed from encoded directories.
    pub project_dir: Option<String>,
    pub timestamp: Option<UsageUnixSeconds>,
    pub provider: UsageProvider,
    pub usage: ParsedUsage,
    pub stop_reason: Option<String>,
}

/// Read only the file selected by the host's authorized native source resolver.
/// Missing/unreadable files return errors; malformed JSONL rows are skipped.
pub fn parse_file(path: &Path) -> io::Result<Vec<ClaudeUsageMessage>> {
    let reader = BufReader::new(File::open(path)?);
    parse_reader(reader, path.file_stem().and_then(|stem| stem.to_str()))
}

/// Parse snapshots, deduplicating by native session and message identity. Claude
/// can write several content blocks for one API response: these are not increments.
pub fn parse_reader(
    mut reader: impl BufRead,
    fallback_session_id: Option<&str>,
) -> io::Result<Vec<ClaudeUsageMessage>> {
    let mut messages: BTreeMap<(Option<String>, String), ClaudeUsageMessage> = BTreeMap::new();
    let mut session_id = fallback_session_id.filter(|id| !id.trim().is_empty()).map(str::to_owned);
    let mut project_dir = None;
    let mut line = Vec::new();
    loop {
        line.clear();
        if reader.read_until(b'\n', &mut line)? == 0 {
            break;
        }
        let Ok(value) = serde_json::from_slice::<Value>(&line) else { continue };
        session_id = label(&value, "sessionId").or(session_id);
        project_dir = label(&value, "cwd").or(project_dir);
        if value.get("type").and_then(Value::as_str) != Some("assistant") {
            continue;
        }
        let Some(message) = value.get("message") else { continue };
        // An explicit contradictory role is not an assistant usage record.
        if message.get("role").is_some_and(|role| role.as_str() != Some("assistant")) {
            continue;
        }
        let Some(message_id) = label(message, "id") else { continue };
        let Some(usage) = parse_response(UsageProtocol::Claude, message) else { continue };
        if !usage.has_billable_tokens() {
            continue;
        }
        let timestamp = value.get("timestamp").and_then(Value::as_str)
            .and_then(|text| chrono::DateTime::parse_from_rfc3339(text).ok())
            .map(|time| UsageUnixSeconds(time.timestamp()));
        let parsed = ClaudeUsageMessage {
            message_id: message_id.clone(),
            session_id: session_id.clone(),
            project_dir: project_dir.clone(),
            timestamp,
            // A Claude model or directory does not identify the actual supplier.
            provider: UsageProvider::Unknown,
            usage,
            stop_reason: label(message, "stop_reason"),
        };
        let key = (session_id.clone(), message_id);
        let replace = match messages.get(&key) {
            None => true,
            Some(previous) => match (previous.stop_reason.is_some(), parsed.stop_reason.is_some()) {
                (false, true) => true,
                (true, false) => false,
                _ => parsed.usage.tokens.output > previous.usage.tokens.output,
            },
        };
        if replace {
            messages.insert(key, parsed);
        }
    }
    Ok(messages.into_values().collect())
}

fn label(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty()).map(str::to_owned)
}
