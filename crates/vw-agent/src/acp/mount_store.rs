//! Versioned, scope-bound skill descriptions. MCP configuration never enters this store.
use crate::session::runtime::SessionScope;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vw_acp::server::BackendError;
use vw_acp::{SessionMountRevision, SessionSkillMount};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MountRecord {
    version: u32,
    binding: String,
    pub revision: SessionMountRevision,
    pub skills: Vec<SessionSkillMount>,
    pub fingerprint: Option<String>,
    mcp_names: Vec<String>,
}

pub(super) fn key(scope: &SessionScope, session: &str) -> String {
    hex::encode(Sha256::digest(
        serde_json::json!([
            scope.tenant().platform_id().as_ref(),
            scope.tenant().company_id().as_ref(),
            scope.directory(),
            session,
        ])
        .to_string()
        .as_bytes(),
    ))
}

pub(super) async fn load(
    scope: &SessionScope,
    session: &str,
) -> Result<Option<MountRecord>, BackendError> {
    let binding = key(scope, session);
    let record: MountRecord = match crate::storage::read(&["acp_session_mounts", &binding]).await {
        Ok(record) => record,
        Err(crate::storage::Error::NotFound(_)) => return Ok(None),
        Err(_) => {
            return Err(BackendError::InvalidRequest("invalid persisted session mounts".into()));
        }
    };
    if record.version != 1
        || record.binding != binding
        || record.skills.is_empty() != record.fingerprint.is_none()
    {
        return Err(BackendError::InvalidRequest(
            "invalid persisted session mounts binding".into(),
        ));
    }
    Ok(Some(record))
}

pub(super) async fn save(
    scope: &SessionScope,
    session: &str,
    prepared: &super::mounts::PreparedMounts,
) -> Result<(), BackendError> {
    prepared
        .skills
        .validate()
        .map_err(|_| BackendError::InvalidRequest("session skill source changed".into()))?;
    let binding = key(scope, session);
    let record = MountRecord {
        version: 1,
        binding: binding.clone(),
        revision: prepared.response.revision,
        skills: prepared.skills.mounts().to_vec(),
        fingerprint: prepared.skills.fingerprint().map(str::to_owned),
        mcp_names: prepared.response.mcp_servers.clone(),
    };
    // Rename is the recovery commit point: before it the old record is authoritative;
    // after it a restarted backend must prepare the complete new revision again.
    crate::storage::write_atomic(&["acp_session_mounts", &binding], &record)
        .await
        .map_err(|_| BackendError::Internal("failed to persist session mounts".into()))
}
