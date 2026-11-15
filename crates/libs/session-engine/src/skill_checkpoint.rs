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
