//! Execution hook requirements and model admission shared by host entry points.
use std::sync::Arc;
use anyhow::{Result, bail};
use agent_config::runtime_schema::Config;
use agent_hooks::HookRunner;

/// Preserve process-pinned requirements when preparing an execution snapshot.
pub fn execution_config(config: &Config) -> Result<Config> {
    let mut config = config.clone();
    let mut required = config.hooks.required.clone();
    if let Some(value) = std::env::var_os("VW_REQUIRED_EXECUTION_HOOKS") {
        let value =
            value.into_string().map_err(|_| anyhow::anyhow!("invalid required hook identity"))?;
        for name in value.split(',') {
            if name.is_empty() {
                bail!("invalid required hook identity");
            }
            required.push(name.to_owned());
        }
    }
    config.hooks.required = required;
    Ok(config)
}

/// Admit a model request without allowing admission hooks to rewrite its payload.
pub async fn authorize_model_request(runner: Option<Arc<HookRunner>>, model: &str) -> Result<()> {
    if let Some(runner) = runner {
        match runner.run_before_llm_call(Vec::new(), model.to_owned()).await {
            agent_hooks::HookResult::Cancel(reason) => bail!("model request denied by hook: {reason}"),
            agent_hooks::HookResult::Continue((messages, selected)) => {
                if !messages.is_empty() || selected != model {
                    bail!("model admission hooks cannot rewrite the request");
                }
            }
        }
    }
    Ok(())
}
