//! 绑定真实宿主HookHost，处理器与执行逻辑由agent-plugins持有。
use crate::tools::context::AgentContextHost;
pub(crate) fn trusted_plugin_hook_handlers(config: &crate::config::Config) -> anyhow::Result<Vec<Box<dyn crate::hooks::HookHandler>>> {
    agent_plugins::hook_handler::trusted_plugin_hook_handlers::<AgentContextHost>(config)
}
pub(crate) fn codex_handlers(name: &str, root: &std::path::Path, manifest: &super::CodexHooksManifest, trust: &super::PluginHookTrustStore) -> anyhow::Result<Vec<Box<dyn crate::hooks::HookHandler>>> {
    agent_plugins::hook_handler::codex_handlers::<AgentContextHost>(name, root, manifest, trust)
}
