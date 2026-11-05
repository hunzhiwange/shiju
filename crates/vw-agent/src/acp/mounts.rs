//! Native session skill sources, retaining their private directory lease.
use crate::session::runtime::{SessionScope, SessionToolset};
use crate::{
    config::Config,
    skills::{
        Skill, SkillLoadMode,
        session_mounts::{SessionSkillBinding, SessionSkillView},
    },
};
use anyhow::{Result, ensure};
use std::{path::Path, sync::Arc};
use vw_acp::server::{BackendError, BackendMountRequest};
use vw_acp::{SessionMountRevision, SessionSkillMount, SetSessionMountsResponse};

/// Audited skill directories owned by one inbound session and retained across tool rounds.
#[derive(Clone, Default)]
pub struct SessionSkillSources {
    view: Option<Arc<SessionSkillView>>,
    mounts: Arc<Vec<SessionSkillMount>>,
    revoked: Arc<std::sync::atomic::AtomicBool>,
}
impl std::fmt::Debug for SessionSkillSources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionSkillSources").field("count", &self.mounts.len()).finish()
    }
}
impl SessionSkillSources {
    pub(super) fn revoke(&self) {
        self.revoked.store(true, std::sync::atomic::Ordering::Release);
    }
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            !self.revoked.load(std::sync::atomic::Ordering::Acquire),
            "session skill mounts revoked"
        );
        if let Some(view) = &self.view {
            view.validate()?;
        }
        Ok(())
    }
    pub(crate) fn extend(
        &self,
        mut skills: Vec<Skill>,
        config: &Config,
        full: bool,
    ) -> Result<Vec<Skill>> {
        self.validate()?;
        if let Some(view) = &self.view {
            for mount in self.mounts.iter() {
                let skill = crate::skills::load_skill_directory(
                    &view.root().join(&mount.name),
                    if full { SkillLoadMode::Full } else { SkillLoadMode::MetadataOnly },
                    &config.skills.security,
                    true,
                )?
                .ok_or_else(|| anyhow::anyhow!("session skill disabled or unavailable"))?;
                ensure!(
                    !skills.iter().any(|local| local
                        .invocation_name()
                        .eq_ignore_ascii_case(&skill.invocation_name())),
                    "session skill conflicts with local source: {}",
                    skill.invocation_name()
                );
                skills.push(skill);
            }
        }
        self.validate()?;
        Ok(skills)
    }
    pub(crate) fn allows_read(&self, path: &Path) -> bool {
        self.validate().is_ok()
            && path.canonicalize().ok().is_some_and(|path| {
                self.mounts.iter().any(|mount| {
                    Path::new(&mount.path)
                        .canonicalize()
                        .ok()
                        .is_some_and(|root| path.starts_with(root))
                })
            })
    }
    pub(crate) fn mounts(&self) -> &[SessionSkillMount] {
        &self.mounts
    }
}

pub(super) struct PreparedMounts {
    pub skills: SessionSkillSources,
    pub tools: SessionToolset,
    pub response: SetSessionMountsResponse,
}
pub(super) async fn prepare(
    scope: &SessionScope,
    session: &str,
    request: BackendMountRequest,
    saved: &[SessionSkillMount],
) -> Result<PreparedMounts, BackendError> {
    prepare_revision(scope, session, request, saved, SessionMountRevision(0)).await
}

pub(super) async fn prepare_revision(
    scope: &SessionScope,
    session: &str,
    request: BackendMountRequest,
    saved: &[SessionSkillMount],
    revision: SessionMountRevision,
) -> Result<PreparedMounts, BackendError> {
    let prepare_error = |error: anyhow::Error| BackendError::InvalidRequest(error.to_string());
    if request.skills.as_ref().is_some_and(|selection| selection.version != 1) {
        return Err(BackendError::InvalidRequest("unsupported session mounts version".into()));
    }
    let mounts = vw_acp::SessionMountSelection::resolve(request.skills.as_ref(), Some(saved));
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
        SessionSkillSources {
            view: Some(Arc::new(view)),
            mounts: Arc::new(mounts.clone()),
            revoked: Default::default(),
        }
    };
    skills
        .extend(crate::skills::load_skills_with_config(scope.directory(), &config), &config, false)
        .map_err(prepare_error)?;
    let tools = super::mcp::prepare_session_tools(&request.mcp_servers).await?;
    skills.validate().map_err(prepare_error)?;
    let names = request
        .mcp_servers
        .iter()
        .map(|server| crate::tools::mcp_common::server_name(server).to_string())
        .collect();
    Ok(PreparedMounts {
        skills,
        tools,
        response: SetSessionMountsResponse {
            revision,
            skills: mounts.iter().map(|m| m.name.clone()).collect(),
            mcp_servers: names,
        },
    })
}
