use agent_config as config;
use serde_json::{json, Value};
use acp_types::PermissionMode;
type ParsedAcpOptions = llm_runtime::acp::options::ParsedAcpOptions<llm_runtime::acp::skill_mounts::MountedSkills>;
struct DefaultsHost;
tokio::task_local! { static STATE: std::cell::RefCell<acp_config::ResolvedAcpxConfig>; }
impl crate::acp_defaults::DefaultsHost for DefaultsHost {
    fn skills(cwd: &std::path::Path, cfg: &config::Config) -> Vec<agent_skills::Skill> {
        agent_skills::catalog::load_skills_with_config(cwd, &cfg.skills, false, |_| Vec::new())
    }
    async fn mcp(_: &std::path::Path) -> anyhow::Result<acp_config::ResolvedAcpxConfig> { Ok(STATE.with(|s| s.borrow().clone())) }
    async fn permission_ceiling(_: &str) -> Result<permission_runtime::next::Ruleset, String> { Ok(permission_runtime::next::from_config(&json!({"*":"allow"}))) }
    async fn preferences() -> anyhow::Result<Value> { Ok(json!({})) }
}
async fn apply_agent_defaults(cfg: &config::Config, options: &Value, backend: &str, cwd: &std::path::Path, parsed: &mut ParsedAcpOptions, messages: &Value) -> Result<Value, llm_runtime::Error> {
    crate::acp_defaults::apply_agent_defaults::<DefaultsHost, _>(cfg, options, backend, cwd, parsed, messages).await
}
async fn isolated(test: impl std::future::Future<Output = ()>) {
    let root = tempfile::tempdir().unwrap();
    let resolved = acp_config::load_resolved_config_from_paths(root.path().join("global.json"), root.path().join("project.json")).await.unwrap();
    STATE.scope(std::cell::RefCell::new(resolved), test).await;
}
mod defaults;
mod skills;
