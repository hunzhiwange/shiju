//! 诊断领域兼容入口及当前请求 Provider 目录组合。
pub use agent_doctor::{DiagResult, Severity, diagnose, run, run_traces};

pub async fn run_models(
    config: &crate::config::Config,
    provider_override: Option<&str>,
    use_cache: bool,
) -> anyhow::Result<()> {
    agent_doctor::run_models(config, provider_override, use_cache, async {
        Ok(crate::provider::provider::list().await?.into_keys().collect())
    }).await
}
