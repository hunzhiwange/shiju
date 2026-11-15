//! Versioned, scope-bound mount references. Resolved MCP configuration stays in memory.
use session_port::SessionScope;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use acp_server::server::BackendError;
use acp_types::{McpCatalogMount, SessionMountRevision, SessionSkillMount};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MountRecord {
    version: u32,
    binding: String,
    pub revision: SessionMountRevision,
    pub skills: Vec<SessionSkillMount>,
    pub fingerprint: Option<String>,
    mcp_names: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_catalog: Vec<McpCatalogMount>,
}

pub fn key(scope: &SessionScope, session: &str) -> String {
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

pub async fn load(
    scope: &SessionScope,
    session: &str,
) -> Result<Option<MountRecord>, BackendError> {
    let binding = key(scope, session);
    let record: MountRecord = match agent_storage::read(&["acp_session_mounts", &binding]).await {
        Ok(record) => record,
        Err(agent_storage::Error::NotFound(_)) => return Ok(None),
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

/// Persist a validated mount generation; the host validates its live skill lease first.
pub async fn save(
    scope: &SessionScope,
    session: &str,
    revision: SessionMountRevision,
    skills: Vec<SessionSkillMount>,
    fingerprint: Option<String>,
    mcp_names: Vec<String>,
    mcp_catalog: Vec<McpCatalogMount>,
) -> Result<(), BackendError> {
    let binding = key(scope, session);
    let record = MountRecord {
        version: 1,
        binding: binding.clone(),
        revision: revision,
        skills: skills,
        fingerprint: fingerprint,
        mcp_names: mcp_names,
        mcp_catalog: mcp_catalog,
    };
    // Rename is the recovery commit point: before it the old record is authoritative;
    // after it a restarted backend must prepare the complete new revision again.
    agent_storage::write_atomic(&["acp_session_mounts", &binding], &record)
        .await
        .map_err(|_| BackendError::Internal("failed to persist session mounts".into()))
}
