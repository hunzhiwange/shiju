//! Typed history anchors copied alongside context checkpoints.
use agent_storage as storage;
use agent_skills::invoked::InvokedSkillRecord;
use serde::{Serialize, Deserialize};
use crate::compaction_checkpoint::{matches_prefix, validate_session};

#[derive(Debug, Serialize, Deserialize)]
pub struct Checkpoint {
    #[serde(default)]
    pub mount_revision: Option<u64>,
    pub source_hashes: Vec<String>,
    pub records: Vec<InvokedSkillRecord>,
}

/// Fork copies source records and anchors only. The destination must still match
/// its retained history and revalidate sources against its current skill policy.
pub async fn copy_scoped(
    source_scope: &str,
    target_scope: &str,
    source: &str,
    target: &str,
    hashes: Option<&[String]>,
) -> Result<(), String> {
    for key in storage::list(&["skill_checkpoint", source_scope, source])
        .await
        .map_err(|e| e.to_string())?
    {
        if key.len() != 5 {
            return Err("invalid skill checkpoint key".into());
        }
        let checkpoint: Checkpoint =
            storage::read(&key.iter().map(String::as_str).collect::<Vec<_>>())
                .await
                .map_err(|e| e.to_string())?;
        if hashes.is_some_and(|hashes| !matches_prefix(hashes, &checkpoint.source_hashes)) {
            continue;
        }
        storage::write_atomic(
            &["skill_checkpoint", target_scope, target, &key[3], &key[4]],
            &checkpoint,
        )
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub async fn remove(scope: &str, session: &str) -> Result<(), String> {
    validate_session(session)?;
    for key in
        storage::list(&["skill_checkpoint", &scope, session]).await.map_err(|e| e.to_string())?
    {
        storage::remove(&key.iter().map(String::as_str).collect::<Vec<_>>())
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

use crate::compaction_checkpoint::message_hash;


// Checkpoint serialization has a single owner in session-engine.
use serde_json::Value;
use sha2::{Digest, Sha256};



fn agent_key(agent: &str) -> String {
    hex::encode(Sha256::digest(agent.as_bytes()))
}

pub async fn save<H: MountRevision>(
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
    let scope = crate::compaction_checkpoint::scope_for_directory(&session_store::ui_store::current_tenant_scope(), &agent_project::instance::directory());
    let agent = agent_key(agent);
    let id = ordered_id::ascending(ordered_id::Prefix::Message, None).map_err(|e| e.to_string())?;
    let checkpoint = Checkpoint {
        mount_revision: H::checkpoint_revision(session).await?,
        source_hashes: history.iter().map(message_hash).collect(),
        records: records.to_vec(),
    };
    storage::write_atomic(&["skill_checkpoint", &scope, session, &agent, &id], &checkpoint)
        .await
        .map_err(|e| e.to_string())
}

/// Only replay snapshots whose complete original prefix still exists. Compaction
/// passes the unmodified source history here, never the generated summary.
pub async fn restore<H: MountRevision>(
    session: &str,
    agent: &str,
    history: &[Value],
) -> Result<Vec<InvokedSkillRecord>, String> {
    validate_session(session)?;
    let scope = crate::compaction_checkpoint::scope_for_directory(&session_store::ui_store::current_tenant_scope(), &agent_project::instance::directory());
    let agent = agent_key(agent);
    let mut keys = storage::list(&["skill_checkpoint", &scope, session, &agent])
        .await
        .map_err(|e| e.to_string())?;
    keys.sort();
    let hashes = history.iter().map(message_hash).collect::<Vec<_>>();
    let mount_revision = H::checkpoint_revision(session).await?;
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


pub trait MountRevision {
    fn checkpoint_revision(session: &str) -> impl std::future::Future<Output = Result<Option<u64>, String>> + Send;
}
