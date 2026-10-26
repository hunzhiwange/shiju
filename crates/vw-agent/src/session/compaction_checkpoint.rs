//! Append-only context checkpoints. Original chat messages are never rewritten.

use crate::storage;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Checkpoint {
    source_hashes: Vec<String>,
    summary: String,
    retained_user: Option<Value>,
}

fn validate_session(session: &str) -> Result<(), String> {
    if session.is_empty() || matches!(session, "." | "..") || session.contains(['/', '\\']) {
        return Err("invalid session ID for context checkpoint".into());
    }
    Ok(())
}

fn scope_for_directory(directory: &str) -> String {
    let tenant = crate::session::ui_store::current_tenant_scope();
    let identity =
        json!([tenant.platform_id().as_ref(), tenant.company_id().as_ref(), directory]).to_string();
    hex::encode(Sha256::digest(identity.as_bytes()))
}

fn message_hash(message: &Value) -> String {
    // Exclude transient timing, model selection and input-source annotations.
    let value = json!({"role": message["role"], "content": message["content"],
        "tool_calls": message["tool_calls"], "tool_call_id": message["tool_call_id"], "name": message["name"]});
    hex::encode(Sha256::digest(value.to_string().as_bytes()))
}

pub(crate) async fn save(
    session: &str,
    source: &[Value],
    summary: &str,
    retained_user: Option<Value>,
) -> Result<(), String> {
    validate_session(session)?;
    if source.is_empty() || summary.trim().is_empty() {
        return Err("cannot persist an empty compaction checkpoint".into());
    }
    let scope = scope_for_directory(&crate::project::instance::directory());
    let id = crate::id::ascending(crate::id::Prefix::Message, None).map_err(|e| e.to_string())?;
    let checkpoint = Checkpoint {
        source_hashes: source.iter().map(message_hash).collect(),
        summary: summary.to_string(),
        retained_user,
    };
    storage::write_atomic(&["context_checkpoint", &scope, session, &id], &checkpoint)
        .await
        .map_err(|e| e.to_string())
}

/// Use only checkpoints whose entire source prefix still matches. A rewind or
/// edited history therefore falls back to an older matching checkpoint or raw history.
pub(crate) async fn restore(session: &str, history: &[Value]) -> Result<Vec<Value>, String> {
    validate_session(session)?;
    let scope = scope_for_directory(&crate::project::instance::directory());
    let mut keys =
        storage::list(&["context_checkpoint", &scope, session]).await.map_err(|e| e.to_string())?;
    if keys.is_empty() {
        return Ok(history.to_vec());
    }
    keys.sort();
    let hashes = history.iter().map(message_hash).collect::<Vec<_>>();
    for key in keys.into_iter().rev() {
        let refs = key.iter().map(String::as_str).collect::<Vec<_>>();
        let checkpoint: Checkpoint = storage::read(&refs).await.map_err(|e| e.to_string())?;
        if !checkpoint.source_hashes.is_empty() && hashes.starts_with(&checkpoint.source_hashes) {
            let mut context = vec![
                json!({"role": "assistant", "content": format!("[Compaction summary]\n{}", checkpoint.summary)}),
            ];
            if let Some(user) = checkpoint.retained_user {
                context.push(user);
            }
            context.extend_from_slice(&history[checkpoint.source_hashes.len()..]);
            return Ok(context);
        }
    }
    Ok(history.to_vec())
}

pub(crate) async fn copy(source: &str, target: &str) -> Result<(), String> {
    validate_session(source)?;
    validate_session(target)?;
    let scope = scope_for_directory(&crate::project::instance::directory());
    for key in
        storage::list(&["context_checkpoint", &scope, source]).await.map_err(|e| e.to_string())?
    {
        let refs = key.iter().map(String::as_str).collect::<Vec<_>>();
        let checkpoint: Checkpoint = storage::read(&refs).await.map_err(|e| e.to_string())?;
        let id = key.last().ok_or_else(|| "invalid checkpoint key".to_string())?;
        storage::write_atomic(&["context_checkpoint", &scope, target, id], &checkpoint)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(crate) async fn remove(session: &str, directory: &str) -> Result<(), String> {
    validate_session(session)?;
    let scope = scope_for_directory(directory);
    for key in
        storage::list(&["context_checkpoint", &scope, session]).await.map_err(|e| e.to_string())?
    {
        storage::remove(&key.iter().map(String::as_str).collect::<Vec<_>>())
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "compaction_checkpoint_tests.rs"]
mod tests;
