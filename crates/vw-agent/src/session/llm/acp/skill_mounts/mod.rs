//! Host tenant injection and composed ACP integration tests.
pub(crate) use llm_runtime::acp::skill_mounts::{MountedSkills, validate, reject_expanded_invocations, prepare_supplier, configure_native_scanner};
pub(super) fn prepare(mounts: &[acp_types::SessionSkillMount], cwd: &std::path::Path, session: &str, security: &config_skills::SkillSecurityConfig) -> Result<Option<MountedSkills>, super::Error> {
    MountedSkills::prepare(mounts, cwd, session, security, &crate::session::ui_store::current_tenant_scope())
}
#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
mod glm_revision;
#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
pub(super) use glm_revision::prepare_glm_revision;

#[cfg(test)]
mod acceptance_tests;
