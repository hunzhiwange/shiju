//! Domain cache and supplier lifecycle contracts migrated from the host.
use crate::acp::{cache::ACP_CLIENT_CACHE, lifecycle, skill_mounts::MountedSkills};
use crate::acp::host_migration_20_tests::{ParsedAcpOptions, get_cached_acp_client};
static CACHE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
fn prepare(mounts: &[acp_types::SessionSkillMount], cwd: &std::path::Path, session: &str, security: &config_skills::SkillSecurityConfig) -> Result<Option<MountedSkills>, crate::Error> {
    MountedSkills::prepare(mounts, cwd, session, security, &vw_api_transport::TenantScope::default())
}
mod session {
    pub(super) use super::get_cached_acp_client;
    pub(super) use crate::acp::session::close_cached_acp_client_if_unshared;
    pub(super) async fn stop_incompatible_cached_clients(agent: &str, cfg: &agent_config::schema::AcpAgentConfig, cwd: &std::path::Path, parsed: &super::ParsedAcpOptions) -> Result<(), crate::Error> {
        crate::acp::session::stop_incompatible_cached_clients::<crate::acp::host_migration_20_tests::CacheHost>(agent, cfg, cwd, parsed).await
    }
}
mod claude;
mod codex;
mod lifecycle_tests;
#[cfg(unix)]
mod profile;
mod driver;
mod acceptance_model_source;
mod acceptance_skill_mounts;
