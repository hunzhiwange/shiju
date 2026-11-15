//! Append-only context checkpoints. Original chat messages are never rewritten.

use agent_storage as storage;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Checkpoint {
    source_hashes: Vec<String>,
    summary: String,
    retained_user: Option<Value>,
}

pub fn validate_session(session: &str) -> Result<(), String> {
    if session.is_empty() || matches!(session, "." | "..") || session.contains(['/', '\\']) {
        return Err("invalid session ID for context checkpoint".into());
    }
    Ok(())
}

pub fn scope_for_directory(tenant: &vw_api_transport::TenantScope, directory: &str) -> String {
    let identity =
        json!([tenant.platform_id().as_ref(), tenant.company_id().as_ref(), directory]).to_string();
    hex::encode(Sha256::digest(identity.as_bytes()))
}

pub fn message_hash(message: &Value) -> String {
    // Exclude transient timing, model selection and input-source annotations.
    let value = json!({"role": message["role"], "content": message["content"],
        "tool_calls": message["tool_calls"], "tool_call_id": message["tool_call_id"], "name": message["name"]});
    hex::encode(Sha256::digest(value.to_string().as_bytes()))
}

pub async fn save(
    scope: &str,
    session: &str,
    source: &[Value],
    summary: &str,
    retained_user: Option<Value>,
) -> Result<(), String> {
    validate_session(session)?;
    if source.is_empty() || summary.trim().is_empty() {
        return Err("cannot persist an empty compaction checkpoint".into());
    }
    let id = ordered_id::ascending(ordered_id::Prefix::Message, None).map_err(|e| e.to_string())?;
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
pub async fn restore(scope: &str, session: &str, history: &[Value]) -> Result<Vec<Value>, String> {
    validate_session(session)?;
    let mut keys =
        storage::list(&["context_checkpoint", &scope, session]).await.map_err(|e| e.to_string())?;
    if keys.is_empty() {
        return normalize_native_history(history);
    }
    keys.sort();
    let hashes = history.iter().map(message_hash).collect::<Vec<_>>();
    for key in keys.into_iter().rev() {
        let refs = key.iter().map(String::as_str).collect::<Vec<_>>();
        let checkpoint: Checkpoint = storage::read(&refs).await.map_err(|e| e.to_string())?;
        if matches_prefix(&hashes, &checkpoint.source_hashes) {
            let mut context = vec![
                json!({"role": "assistant", "content": format!("[Compaction summary]\n{}", checkpoint.summary)}),
            ];
            if let Some(user) = checkpoint.retained_user {
                context.push(user);
            }
            context.extend_from_slice(&history[checkpoint.source_hashes.len()..]);
            return normalize_native_history(&context);
        }
    }
    normalize_native_history(history)
}

pub async fn copy_fork(source_scope: &str, target_scope: &str, source: &str, target: &str, history: &[Value]) -> Result<(), String> {
    let hashes = history.iter().map(message_hash).collect::<Vec<_>>();
    copy_scoped(source_scope, target_scope, source, target, Some(&hashes)).await
}

pub fn matches_prefix(hashes: &[String], checkpoint: &[String]) -> bool {
    !checkpoint.is_empty() && hashes.starts_with(checkpoint)
}

pub async fn copy_scoped(
    source_scope: &str,
    target_scope: &str,
    source: &str,
    target: &str,
    hashes: Option<&[String]>,
) -> Result<(), String> {
    validate_session(source)?;
    validate_session(target)?;
    super::skill_checkpoint::copy_scoped(source_scope, target_scope, source, target, hashes)
        .await?;
    for key in storage::list(&["context_checkpoint", source_scope, source])
        .await
        .map_err(|e| e.to_string())?
    {
        let refs = key.iter().map(String::as_str).collect::<Vec<_>>();
        let checkpoint: Checkpoint = storage::read(&refs).await.map_err(|e| e.to_string())?;
        if hashes.is_some_and(|hashes| !matches_prefix(hashes, &checkpoint.source_hashes)) {
            continue;
        }
        let id = key.last().ok_or_else(|| "invalid checkpoint key".to_string())?;
        storage::write_atomic(&["context_checkpoint", target_scope, target, id], &checkpoint)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub async fn remove_strict(scope: &str, session: &str) -> Result<(), String> {
    validate_session(session)?;
    for prefix in ["skill_checkpoint", "context_checkpoint"] {
        for key in storage::list_strict(&[prefix, &scope, session])
            .await
            .map_err(|error| error.to_string())?
        {
            storage::remove(&key.iter().map(String::as_str).collect::<Vec<_>>())
                .await
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

pub async fn remove(scope: &str, session: &str) -> Result<(), String> {
    super::skill_checkpoint::remove(scope, session).await?;
    validate_session(session)?;
    for key in
        storage::list(&["context_checkpoint", &scope, session]).await.map_err(|e| e.to_string())?
    {
        storage::remove(&key.iter().map(String::as_str).collect::<Vec<_>>())
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}


fn normalize_native_history(history: &[Value]) -> Result<Vec<Value>, String> {
    #[cfg(not(target_arch = "wasm32"))]
    return crate::native_history::normalize_history(history).map_err(|e| e.to_string());
    #[cfg(target_arch = "wasm32")]
    Ok(history.to_vec())
}
