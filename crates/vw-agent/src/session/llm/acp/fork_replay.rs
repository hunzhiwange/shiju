//! Host project/fork validation before opening durable delivery evidence.
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};

pub(super) struct ForkReplay(llm_runtime::acp::fork_replay::ForkReplay);
impl ForkReplay {
    pub(super) fn into_runtime(self) -> llm_runtime::acp::fork_replay::ForkReplay { self.0 }
    pub(super) async fn load(session: &str, directory: &str) -> anyhow::Result<Option<Self>> {
        if session.is_empty() { return Ok(None); }
        let Some(project) = crate::project::instance::project() else { return Ok(None); };
        if crate::session::fork_execution::load_scoped(&project.id, session).await?.is_none() { return Ok(None); }
        let store = GatewaySettingsStore::new(GlobalDatabase::for_home(&crate::global::paths().home)?)?;
        let tenant = crate::session::ui_store::current_tenant_scope();
        Ok(Some(Self(llm_runtime::acp::fork_replay::ForkReplay::load(store, tenant, session, directory)?)))
    }
}
impl std::ops::Deref for ForkReplay {
    type Target = llm_runtime::acp::fork_replay::ForkReplay;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl std::ops::DerefMut for ForkReplay {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}
#[cfg(test)]
#[path = "fork_replay_tests.rs"]
mod tests;
