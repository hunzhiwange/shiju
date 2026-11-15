//! Skill settings reducers. The host executes network and cross-page effects.
use desktop_provider_apps::AgentAppsContext;
use vw_api_app_skills::bindings_api::SkillBindingTarget;

/// Explicit scope and authorization readback; no credentials enter domain events.
pub struct Input {
    pub connection_error: String,
    pub context: Option<AgentAppsContext>,
    pub project: Option<String>,
    pub directory: Option<String>,
    pub host: String,
    pub catalog: u64,
    pub authorized: bool,
    pub loopback: bool,
    pub targets: Vec<SkillBindingTarget>,
    pub repository_version: u64,
    pub job_active: bool,
}
pub mod backup_modal;
pub mod bindings_view;
pub mod claude_view;
pub mod storage_modal;
pub mod updates_view;
pub mod install_view;
pub mod jobs_view;
pub mod market_view;
pub mod repo_modal;
pub mod import_modal;
pub mod state;
pub mod catalog;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod deeplink;
mod catalog_mapping;
