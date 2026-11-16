//! Host resolution of trusted ACP agent defaults.
#[cfg(test)]
use acp_types::PermissionMode;
use serde_json::Value;
use crate::config;
use super::{Error, ParsedAcpOptions};
pub(crate) use llm_runtime::acp::config::{normalize_acp_agent_config, build_acp_command_line, lookup_acp_command};

/// Resolve defaults through the session engine with host-scoped IO.
pub(super) async fn apply_agent_defaults(
    cfg: &config::schema::Config, options: &Value, backend: &str,
    cwd: &std::path::Path, parsed: &mut ParsedAcpOptions, messages: &Value,
) -> Result<Value, Error> {
    session_engine::acp_defaults::apply_agent_defaults::<DefaultsHost, _>(cfg, options, backend, cwd, parsed, messages).await
}
struct DefaultsHost;
impl session_engine::acp_defaults::DefaultsHost for DefaultsHost {
    fn skills(cwd: &std::path::Path, cfg: &config::schema::Config) -> Vec<crate::skills::Skill> { crate::skills::load_skills_with_config(cwd, cfg) }
    async fn mcp(cwd: &std::path::Path) -> anyhow::Result<acp_config::ResolvedAcpxConfig> { crate::tools::mcp_common::load_resolved_acp_config_from(cwd).await }
    async fn permission_ceiling(key: &str) -> Result<permission_runtime::next::Ruleset, String> { crate::agent::native_permission_ceiling(key).await }
    async fn preferences() -> anyhow::Result<Value> {
        let tenant = crate::session::ui_store::current_tenant_scope();
        Ok(tokio::task::spawn_blocking(move || crate::session::ui_config::load_app_config_result(&tenant)).await??)
    }
}
#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;

pub(crate) fn parse_acp_options(options: &Value, agent: &str, config: &config::schema::AcpAgentConfig) -> Result<ParsedAcpOptions, Error> {
    llm_runtime::acp::config::parse_acp_options(options, agent, config)
}
