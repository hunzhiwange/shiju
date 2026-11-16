//! Native session skill sources, retaining their private directory lease.
use crate::agent_apps::mcp::acp_mount::ResolvedCatalogMounts;
use crate::session::runtime::{SessionScope, SessionToolset};
use crate::{
    skills::session_mounts::{SessionSkillBinding, SessionSkillView},
};
use anyhow::Result;
use acp_server::server::{BackendError, BackendMountRequest};
use acp_types::{McpCatalogMount, SessionMountRevision, SessionSkillMount, SetSessionMountsResponse};

pub use agent_skills::session_sources::SessionSkillSources;

pub(super) struct PreparedMounts {
    pub skills: SessionSkillSources,
    pub tools: SessionToolset,
    pub response: SetSessionMountsResponse,
    pub mcp_catalog: Vec<McpCatalogMount>,
}
pub(super) async fn prepare(
    scope: &SessionScope,
    session: &str,
    request: BackendMountRequest,
    saved: &[SessionSkillMount],
) -> Result<PreparedMounts, BackendError> {
    prepare_revision(scope, session, request, saved, SessionMountRevision(0), Default::default())
        .await
}

pub(super) async fn prepare_revision(
    scope: &SessionScope,
    session: &str,
    mut request: BackendMountRequest,
    saved: &[SessionSkillMount],
    revision: SessionMountRevision,
    managed: ResolvedCatalogMounts,
) -> Result<PreparedMounts, BackendError> {
    let prepare_error = |error: anyhow::Error| BackendError::InvalidRequest(error.to_string());
    if request.skills.as_ref().is_some_and(|selection| selection.version != 1) {
        return Err(BackendError::InvalidRequest("unsupported session mounts version".into()));
    }
    let mounts = acp_types::SessionMountSelection::resolve(request.skills.as_ref(), Some(saved));
    let config = crate::config::get_project_for_tenant(scope.tenant(), scope.directory())
        .await
        .map_err(|error| BackendError::InvalidRequest(error.to_string()))?;
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
            &config.skills.security,
        )
        .map_err(prepare_error)?;
        SessionSkillSources::new(view, mounts.clone())
    };
    skills
        .extend(crate::skills::load_skills_with_config(scope.directory(), &config), &config.skills.security, false)
        .map_err(prepare_error)?;
    let ResolvedCatalogMounts { leases, servers } = managed;
    request.mcp_servers.extend(servers);
    let mcp_catalog = leases.as_ref().map(|leases| leases.mounts.clone()).unwrap_or_default();
    let tools = super::mcp::prepare_session_tools(&request.mcp_servers, leases).await?;
    skills.validate().map_err(prepare_error)?;
    let names = request
        .mcp_servers
        .iter()
        .map(|server| crate::tools::mcp_common::server_name(server).to_string())
        .collect();
    Ok(PreparedMounts {
        skills,
        tools,
        mcp_catalog,
        response: SetSessionMountsResponse {
            revision,
            skills: mounts.iter().map(|m| m.name.clone()).collect(),
            mcp_servers: names,
        },
    })
}

/// Checkpoints from an earlier committed mount generation must never replay.
pub(crate) async fn checkpoint_revision(session: &str) -> Result<Option<u64>, String> {
    let directory = crate::project::instance::directory();
    // Native checkpoint callers without a project context have no inbound mounts.
    if directory.is_empty() {
        return Ok(None);
    }
    let scope = SessionScope::new(crate::session::ui_store::current_tenant_scope(), directory)
        .map_err(|error| error.to_string())?;
    super::mount_store::load(&scope, session)
        .await
        .map(|record| record.map(|record| record.revision.0))
        .map_err(|_| "cannot validate skill checkpoint mount revision".into())
}
