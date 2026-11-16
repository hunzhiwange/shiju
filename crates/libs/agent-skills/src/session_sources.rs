//! Session skill leases and revocation state shared by native execution and ACP.
use crate::{Skill, session_mounts::SessionSkillView};
use acp_types::SessionSkillMount;
use anyhow::{Result, ensure};
use std::{path::Path, sync::Arc};

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
    pub fn new(view: SessionSkillView, mounts: Vec<SessionSkillMount>) -> Self {
        Self { view: Some(Arc::new(view)), mounts: Arc::new(mounts), revoked: Default::default() }
    }
    pub fn revoke(&self) {
        self.revoked.store(true, std::sync::atomic::Ordering::Release);
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.revoked.load(std::sync::atomic::Ordering::Acquire),
            "session skill mounts revoked"
        );
        if let Some(view) = &self.view {
            view.validate()?;
        }
        Ok(())
    }
    pub fn extend(
        &self,
        skills: Vec<Skill>,
        security: &config_skills::SkillSecurityConfig,
        full: bool,
    ) -> Result<Vec<Skill>> {
        crate::catalog::extend_session_skills(self, skills, security, full)
    }
    pub fn allows_read(&self, path: &Path) -> bool {
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
    pub fn fingerprint(&self) -> Option<&str> {
        self.view.as_ref().map(|view| view.fingerprint())
    }
    pub fn mounts(&self) -> &[SessionSkillMount] {
        &self.mounts
    }
}

impl crate::catalog::SessionSkillInventory for SessionSkillSources {
    fn validate(&self) -> Result<()> {
        SessionSkillSources::validate(self)
    }

    fn view(&self) -> Option<&SessionSkillView> {
        self.view.as_deref()
    }

    fn mounts(&self) -> &[SessionSkillMount] {
        &self.mounts
    }
}
