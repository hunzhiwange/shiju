//! ACP 技能挂载准备；原生目录发现和配置由宿主注入。
use session_port::SessionScope;
use agent_skills::{Skill, session_sources::SessionSkillSources, session_mounts::{SessionSkillBinding, SessionSkillView}};
use acp_server::server::{BackendError, BackendMountRequest};
use acp_types::{McpCatalogMount, SessionMountRevision, SessionSkillMount, SetSessionMountsResponse};
use crate::mcp::SessionToolset;

pub struct PreparedMounts {
    pub skills: SessionSkillSources,
    pub tools: SessionToolset,
    pub response: SetSessionMountsResponse,
    pub mcp_catalog: Vec<McpCatalogMount>,
}
pub fn prepare_skills(
    scope: &SessionScope,
    session: &str,
    request: &BackendMountRequest,
    saved: &[SessionSkillMount],
    revision: SessionMountRevision,
    security: &config_skills::SkillSecurityConfig,
    native_skills: impl FnOnce() -> Vec<Skill>,
) -> Result<(SessionSkillSources, Vec<SessionSkillMount>), BackendError> {
    let prepare_error = |error: anyhow::Error| BackendError::InvalidRequest(error.to_string());
    if request.skills.as_ref().is_some_and(|selection| selection.version != 1) {
        return Err(BackendError::InvalidRequest("unsupported session mounts version".into()));
    }
    let mounts = acp_types::SessionMountSelection::resolve(request.skills.as_ref(), Some(saved));
    let skills = if mounts.is_empty() {
        SessionSkillSources::default()
    } else {
        let view = SessionSkillView::prepare(
            SessionSkillBinding {
                tenant: serde_json::json!([
                    scope.tenant().platform_id().as_ref(),
                    scope.tenant().company_id().as_ref()
                ])
                .to_string(),
                project: scope.directory().to_path_buf(),
                session: session.into(),
                revision: revision.0,
            },
            &mounts,
            security,
        )
        .map_err(prepare_error)?;
        SessionSkillSources::new(view, mounts.clone())
    };
    skills
        .extend(native_skills(), security, false)
        .map_err(prepare_error)?;
    Ok((skills, mounts))
}

pub async fn save(
    scope: &SessionScope,
    session: &str,
    prepared: &PreparedMounts,
) -> Result<(), BackendError> {
    prepared
        .skills
        .validate()
        .map_err(|_| BackendError::InvalidRequest("session skill source changed".into()))?;
    crate::mount_store::save(
        scope,
        session,
        prepared.response.revision,
        prepared.skills.mounts().to_vec(),
        prepared.skills.fingerprint().map(str::to_owned),
        prepared.response.mcp_servers.clone(),
        prepared.mcp_catalog.clone(),
    ).await
}
