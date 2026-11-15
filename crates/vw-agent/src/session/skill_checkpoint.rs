//! History-anchored invocation sources. Expanded skill bodies are never persisted.

use super::compaction_checkpoint::{
    matches_prefix, message_hash, scope_for_directory, validate_session,
};
use crate::skills::InvokedSkillRecord;
use crate::storage;
// Checkpoint serialization has a single owner in session-engine.
use serde_json::Value;
use sha2::{Digest, Sha256};

use session_engine::skill_checkpoint::Checkpoint;

fn agent_key(agent: &str) -> String {
    hex::encode(Sha256::digest(agent.as_bytes()))
}

pub(crate) async fn save(
    session: &str,
    agent: &str,
    history: &[Value],
    records: &[InvokedSkillRecord],
) -> Result<(), String> {
    validate_session(session)?;
    if records.is_empty() {
        return Ok(());
    }
    if history.is_empty() {
        return Err("cannot persist skill invocations without a history anchor".into());
    }
    let scope = scope_for_directory(&crate::project::instance::directory());
    let agent = agent_key(agent);
    let id = crate::id::ascending(crate::id::Prefix::Message, None).map_err(|e| e.to_string())?;
    let checkpoint = Checkpoint {
        mount_revision: crate::acp::mounts::checkpoint_revision(session).await?,
        source_hashes: history.iter().map(message_hash).collect(),
        records: records.to_vec(),
    };
    storage::write_atomic(&["skill_checkpoint", &scope, session, &agent, &id], &checkpoint)
        .await
        .map_err(|e| e.to_string())
}

/// Only replay snapshots whose complete original prefix still exists. Compaction
/// passes the unmodified source history here, never the generated summary.
pub(crate) async fn restore(
    session: &str,
    agent: &str,
    history: &[Value],
) -> Result<Vec<InvokedSkillRecord>, String> {
    validate_session(session)?;
    let scope = scope_for_directory(&crate::project::instance::directory());
    let agent = agent_key(agent);
    let mut keys = storage::list(&["skill_checkpoint", &scope, session, &agent])
        .await
        .map_err(|e| e.to_string())?;
    keys.sort();
    let hashes = history.iter().map(message_hash).collect::<Vec<_>>();
    let mount_revision = crate::acp::mounts::checkpoint_revision(session).await?;
    for key in keys.into_iter().rev() {
        let checkpoint: Checkpoint =
            storage::read(&key.iter().map(String::as_str).collect::<Vec<_>>())
                .await
                .map_err(|e| e.to_string())?;
        if checkpoint.mount_revision == mount_revision
            && matches_prefix(&hashes, &checkpoint.source_hashes)
        {
            return Ok(checkpoint.records);
        }
    }
    Ok(Vec::new())
}

pub(super) async fn copy_scoped(source_scope: &str, target_scope: &str, source: &str, target: &str, hashes: Option<&[String]>) -> Result<(), String> {
    session_engine::skill_checkpoint::copy_scoped(source_scope, target_scope, source, target, hashes).await
}
pub(crate) async fn remove(session: &str, directory: &str) -> Result<(), String> {
    session_engine::skill_checkpoint::remove(&scope_for_directory(directory), session).await
}

#[cfg(test)]
#[path = "skill_checkpoint_tests.rs"]
mod tests;
