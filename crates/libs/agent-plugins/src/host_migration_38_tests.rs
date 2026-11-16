//! Trusted command hook acceptance, owned by the handler implementation.
mod host;
mod tools {
 pub type ToolUseContext = tool_context::context::ToolUseContext<super::host::Host>;
 pub use tool_context::context::scope_tool_use_context;
}
mod hooks {
 pub use agent_hooks::{HookResult, HookRunner};
 pub fn new_hook_runner() -> HookRunner { HookRunner::new() }
}
mod plugins {
 pub use crate::*;
 pub mod hook_handler {
  pub fn codex_handlers(name: &str, root: &std::path::Path, manifest: &plugin_types::CodexHooksManifest, trust: &crate::PluginHookTrustStore) -> anyhow::Result<Vec<Box<dyn agent_hooks::HookHandler>>> {
   crate::hook_handler::codex_handlers::<super::super::host::Host>(name, root, manifest, trust)
  }
 }
 pub fn trusted_plugin_hook_handlers(config: &agent_config::runtime::Config) -> anyhow::Result<Vec<Box<dyn agent_hooks::HookHandler>>> { crate::hook_handler::trusted_plugin_hook_handlers::<super::host::Host>(config) }
}
use plugin_hooks::hook_manifest;
mod hook_command {
 pub fn bind_input(payload: &mut serde_json::Value, event: plugin_types::PluginHookEvent) -> anyhow::Result<()> { plugin_hooks::hook_command::bind_input::<super::host::Host>(payload, event) }
 pub fn retain_context(text: &str, limit: Option<usize>, data: &std::path::Path) -> anyhow::Result<()> { plugin_hooks::hook_command::retain_context::<super::host::Host>(text, limit, data) }
}
mod hook_session {
 pub async fn start(context: std::sync::Arc<super::tools::ToolUseContext>, resumed: bool) -> anyhow::Result<()> { plugin_hooks::hook_session::start::<super::host::Host>(context, resumed).await }
 pub async fn close(session: &str, root: &std::path::Path) { plugin_hooks::hook_session::close::<super::host::Host>(session, root).await }
}
mod hook_prompt {
 pub async fn submit(context: std::sync::Arc<super::tools::ToolUseContext>, prompt: &str) -> anyhow::Result<()> { plugin_hooks::hook_prompt::submit::<super::host::Host>(context, prompt).await }
}
mod hook_model {
 pub async fn emit(status: Option<&str>, usage: serde_json::Value) { plugin_hooks::hook_model::emit::<super::host::Host>(status, usage).await }
}
mod hook_stop {
 pub async fn emit(context: std::sync::Arc<super::tools::ToolUseContext>, interrupt: bool, active: bool, message: Option<&str>) -> anyhow::Result<plugin_hooks::hook_stop::Outcome> { plugin_hooks::hook_stop::emit::<super::host::Host>(context, interrupt, active, message).await }
}
mod t32_tests;
mod t33_tests;
mod t34_tests;
mod t35_tests;
mod t37_tests;
mod t39_tests;
mod t40_tests;
mod t41_tests;
mod t44_tests;
mod platform_tests;
