//! Host composition for trusted plugin hooks and execution context admission.
use std::sync::Arc;
use anyhow::Result;
use crate::config::Config;
use super::HookRunner;
use super::builtin::CommandAuditStore;
pub use agent_hooks::HookRuntimeCandidate;
#[derive(Clone)]
pub struct HookRuntime(agent_hooks::HookRuntime);
impl HookRuntime {
    pub fn build(config: &Config) -> Result<Self> {
        agent_hooks::HookRuntime::build(&config.hooks, || crate::plugins::trusted_plugin_hook_handlers(config), Arc::new(NativeHookCleanup)).map(Self)
    }
    pub fn build_candidate(&self, config: &Config) -> Result<HookRuntimeCandidate> {
        self.0.build_candidate(&config.hooks, || crate::plugins::trusted_plugin_hook_handlers(config), Arc::new(NativeHookCleanup))
    }
    pub fn replace(&self, candidate: HookRuntimeCandidate) { self.0.replace(candidate); }
    pub fn runner_snapshot(&self) -> Option<Arc<HookRunner>> { self.0.runner_snapshot() }
    pub fn audit_store(&self) -> CommandAuditStore { self.0.audit_store() }
}
pub fn build_hook_runtime(config: &Config) -> Result<HookRuntime> { HookRuntime::build(config) }

/// Build execution gates, retaining process-pinned requirements across config edits.
pub(crate) fn execution_hook_runner(config: &Config) -> Result<Option<Arc<HookRunner>>> {
    let config = tool_assembly::hooks::execution_config(config)?;
    Ok(HookRuntime::build(&config)?.runner_snapshot())
}

/// Check model admission immediately before a provider request, without exposing prompts.
pub(crate) async fn authorize_model_request(config: &Config, model: &str) -> Result<bool> {
    let (runner, required) = if let Some(context) = crate::tools::current_tool_use_context()
        && let Some(snapshot) = context.plugin_snapshot()
    {
        snapshot.validate_active()?;
        (snapshot.hooks.clone(), snapshot.required_hooks)
    } else {
        (
            execution_hook_runner(config)?,
            !config.hooks.required.is_empty()
                || std::env::var_os("VW_REQUIRED_EXECUTION_HOOKS").is_some(),
        )
    };
    tool_assembly::hooks::authorize_model_request(runner, model).await?;
    Ok(required)
}

/// Resource ownership remains in the host; ordering belongs to the runner.
struct NativeHookCleanup;
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl agent_hooks::HookCleanup for NativeHookCleanup {
    async fn gateway_stop(&self) {
        #[cfg(unix)]
        if let Some(runtime) = crate::tools::computer_control::NativeControlRuntime::shared() {
            runtime.stop_all().await;
        }
    }
    async fn session_end(&self, session_id: &str) {
        #[cfg(unix)]
        if let Some(context) = crate::tools::current_tool_use_context()
            && context.session() == session_id
            && let Some(runtime) = crate::tools::computer_control::NativeControlRuntime::shared()
        {
            runtime.stop_context(&context).await;
        }
        #[cfg(not(unix))]
        let _ = session_id;
    }
}

/// 创建带宿主资源清理的钩子运行器。
pub fn new_hook_runner() -> HookRunner {
    let mut runner = HookRunner::new();
    runner.set_cleanup(Arc::new(NativeHookCleanup));
    runner
}
