//! 启动时的工具、技能与 hook 工厂，以及配置监听器的窄端口。
use std::{path::Path, sync::Arc};
use agent_config::runtime_schema::{AutonomyConfig, Config};
use anyhow::Result;

pub trait ChannelStartupHost: crate::host::ChannelHost {
    fn tenant(&self) -> vw_api_transport::id::TenantScope;
    fn tools(&self, config: &Config, security: &Arc<agent_security::SecurityPolicy>, runtime: Arc<dyn runtime_types::RuntimeAdapter>, memory: Arc<dyn agent_memory::Memory>) -> Vec<Box<dyn tool_types::Tool>>;
    fn skills(&self, workspace: &Path, config: &Config) -> Vec<agent_skills::Skill>;
    fn hooks(&self, config: &Config) -> Result<Option<Arc<agent_hooks::HookRunner>>>;
    fn recursive_policy(&self) -> &'static str {
        agent_loop::prompt::multi_agent::recursive_orchestration_policy()
    }
    fn tool_instructions(&self, specs: &[tool_types::ToolSpec]) -> String {
        agent_loop::instructions::build_tool_instructions_from_specs(specs)
    }
    fn shell_policy(&self, autonomy: &AutonomyConfig) -> String {
        agent_loop::instructions::build_shell_policy_instructions(autonomy)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait::async_trait]
pub trait ChannelConfigSource: Send {
    async fn poll(&mut self, config: &Config) -> Result<Option<agent_config::runtime_schema::ChannelsConfig>>;
}
