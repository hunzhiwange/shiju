//! Scope-local Pi child projection. Supply the complete authorized file set and
//! the persisted first-import cutoff. Replace the previous projection atomically;
//! never append this snapshot or advance cursors after an error.
use std::{
    collections::{BTreeMap, HashSet},
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::pi::{self, PiFileRevision, PiUsageMessage};

#[derive(Debug)]
pub struct PiChildrenSnapshot {
    pub revisions: BTreeMap<PathBuf, PiFileRevision>,
    pub messages: Vec<PiUsageMessage>,
    pub incomplete_files: Vec<PathBuf>,
}

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Invalid Pi child session")
}

/// Linked paths are evidence only: they are never opened or added to the
/// authorized set. Missing/out-of-scope children retain their parent projection.
pub fn projection_session_files(entry: &Value) -> Vec<&str> {
    let mut links = Vec::new();
    if entry["type"] == "message" && entry["message"]["role"] == "toolResult"
        && entry["message"].get("usage").is_some()
    {
        collect_links(&entry["message"]["details"], &mut links);
    }
    links
}

fn collect_links<'a>(value: &'a Value, links: &mut Vec<&'a str>) {
    match value {
        Value::Object(fields) => for (key, value) in fields {
            if key == "sessionFile" {
                if let Some(path) = value.as_str().filter(|s| !s.is_empty()) { links.push(path); }
            } else { collect_links(value, links); }
        },
        Value::Array(items) => for value in items { collect_links(value, links); },
        _ => {}
    }
}

/// Only the official single/parallel/chain contract carries embedded requests.
/// Linked extensions use their session files/top-level usage instead.
pub fn parse_subagent_records(entry: &Value) -> Option<Vec<Value>> {
    let message = &entry["message"];
    let details = &message["details"];
    if entry["type"] != "message" || message["role"] != "toolResult"
        || message["toolName"] != "subagent" || message.get("usage").is_some()
        || !matches!(details["mode"].as_str(), Some("single" | "parallel" | "chain"))
    { return None; }
    let mut links = Vec::new();
    collect_links(details, &mut links);
    if !links.is_empty() { return None; }
    let results = details["results"].as_array()?;
    // Exclude mutable details/usage from native parent identity.
    let parent = entry.get("id").and_then(Value::as_str).filter(|s| !s.is_empty())
        .map(str::to_owned).unwrap_or_else(|| semantic_key(entry));
    let mut records = Vec::new();
    for (result_index, result) in results.iter().enumerate() {
        let mut has_requests = false;
        if let Some(messages) = result["messages"].as_array() {
            for (message_index, child) in messages.iter().enumerate() {
                if child["role"] != "assistant" || !child["usage"].is_object() { continue; }
                has_requests = true;
                records.push(json!({
                    "type": "message",
                    "id": format!("{parent}:result:{result_index}:message:{message_index}"),
                    "timestamp": child.get("timestamp").or_else(|| entry.get("timestamp")),
                    "message": child,
                }));
            }
        }
        if has_requests { continue; }
        if let Some(usage) = result.get("usage").filter(|v| v.is_object()) {
            let mut usage = usage.clone();
            usage["cost"] = json!({"total": usage.get("cost")});
            records.push(json!({
                "type": "message",
                "id": format!("{parent}:result:{result_index}:aggregate"),
                "timestamp": entry.get("timestamp"),
                "message": {"role": "assistant", "model": result.get("model"),
                    "stopReason": result.get("stopReason"), "usage": usage},
            }));
        }
    }
    Some(records)
}

fn semantic_key(entry: &Value) -> String {
    let mut semantic = entry.clone();
    if let Some(object) = semantic.as_object_mut() { object.remove("id"); }
    format!("{:x}", Sha256::digest(pi::canonical_json(&semantic).to_string().as_bytes()))
}

/// Reparse all revisions so same-size repairs, cutoff changes and changes to
/// child availability invalidate the projection together. One scope per call.
/// Nested children follow the upstream extension-less parent directory rule.
pub fn project_files(files: &[PathBuf], child_cutoff: i64) -> io::Result<PiChildrenSnapshot> {
    let imported: HashSet<PathBuf> = files.iter().map(std::fs::canonicalize)
        .collect::<io::Result<_>>()?;
    let stems: HashSet<_> = imported.iter().map(|p| p.with_extension("")).collect();
    let mut snapshot = PiChildrenSnapshot {
        revisions: BTreeMap::new(), messages: Vec::new(), incomplete_files: Vec::new(),
    };
    let mut requests = BTreeMap::new();
    // Semantic dedup applies when either copy lacks an entry ID. Two distinct
    // native IDs remain separate requests, even with identical token counts.
    let mut semantics: BTreeMap<String, Vec<(bool, String)>> = BTreeMap::new();
    let mut paths: Vec<_> = imported.iter().collect();
    paths.sort();
    for path in paths {
        let nested = path.ancestors().skip(1).any(|p| stems.contains(p));
        let mut file = File::open(path)?;
        let before = pi::revision(&mut file)?;
        file.seek(SeekFrom::Start(0))?;
        let mut bytes = Vec::new();
        Read::by_ref(&mut file).take(before.file_size).read_to_end(&mut bytes)?;
        if pi::revision(&mut file)? != before { return Err(changed()); }
        let mut header = None;
        let mut entries = 0;
        for line in bytes.split_inclusive(|b| *b == b'\n') {
            entries += 1;
            if entries > 500_001 { return Err(invalid()); }
            if line.last() != Some(&b'\n') {
                snapshot.incomplete_files.push(path.clone());
                break;
            }
            let Ok(entry) = serde_json::from_slice::<Value>(line) else { continue; };
            if header.is_none() {
                // Let the shared main parser enforce the header contract.
                pi::parse_reader(line)?;
                header = Some(entry);
                continue;
            }
            let records = parse_subagent_records(&entry).unwrap_or_else(|| vec![entry]);
            for entry in records {
                let text = format!("{}\n{}\n", header.as_ref().ok_or_else(invalid)?, entry);
                for message in pi::parse_reader(text.as_bytes())?.0 {
                    // Missing timestamps cannot safely select either cutoff side.
                    let time = message.timestamp.ok_or_else(invalid)?.0;
                    if nested && time < child_cutoff { continue; }
                    let links = projection_session_files(&entry);
                    if time >= child_cutoff && !links.is_empty() && links.iter().all(|link| {
                        std::fs::canonicalize(Path::new(link)).is_ok_and(|p| imported.contains(&p))
                    }) { continue; }
                    let semantic = semantic_key(&entry);
                    let native = links.is_empty()
                        && entry["id"].as_str().is_some_and(|s| !s.is_empty());
                    let copies = semantics.entry(semantic).or_default();
                    if copies.iter().any(|(has_id, id)| {
                        id != &message.request_id && (!native || !has_id)
                    }) { continue; }
                    copies.push((native, message.request_id.clone()));
                    requests.insert(message.request_id.clone(), message);
                }
            }
        }
        if header.is_none() && !snapshot.incomplete_files.contains(path) { return Err(invalid()); }
        snapshot.revisions.insert(path.clone(), before);
    }
    // Detect child rewrites during a parent's parse before publishing anything.
    for (path, expected) in &snapshot.revisions {
        if pi::revision(&mut File::open(path)?)? != *expected { return Err(changed()); }
    }
    snapshot.messages = requests.into_values().collect();
    Ok(snapshot)
}

fn changed() -> io::Error {
    io::Error::new(io::ErrorKind::Interrupted, "Pi sessions changed during projection")
}
