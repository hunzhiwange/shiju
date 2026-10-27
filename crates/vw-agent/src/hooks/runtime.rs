//! 可热替换的钩子运行时，保证候选构建完成后再原子发布。
//!
//! 命令审计存储跨替换保持稳定；候选构建失败不会影响当前正在服务的 runner 快照。

use std::sync::{Arc, RwLock};

use anyhow::{Context, Result, bail};

use crate::app::agent::config::Config;

use super::HookRunner;
use super::builtin::{CommandAuditStore, CommandLoggerHook};

/// 已完整构建、可原子替换当前 runner 的候选对象。
pub struct HookRuntimeCandidate {
    runner: Option<Arc<HookRunner>>,
}

/// 具有稳定审计存储和可替换 runner 快照的共享钩子运行时。
#[derive(Clone)]
pub struct HookRuntime {
    runner: Arc<RwLock<Option<Arc<HookRunner>>>>,
    audit_store: CommandAuditStore,
}

impl HookRuntime {
    /// 构建初始运行时，不发布未完成装配的 runner。
    pub fn build(config: &Config) -> Result<Self> {
        let audit_store = CommandAuditStore::default();
        let candidate = build_candidate(config, audit_store.clone())?;
        Ok(Self { runner: Arc::new(RwLock::new(candidate.runner)), audit_store })
    }

    /// 在当前 runner 保持活动时构建替换候选。
    pub fn build_candidate(&self, config: &Config) -> Result<HookRuntimeCandidate> {
        build_candidate(config, self.audit_store.clone())
    }

    /// 在写锁保护下发布完整候选。
    pub fn replace(&self, candidate: HookRuntimeCandidate) {
        *self.runner.write().unwrap_or_else(|error| error.into_inner()) = candidate.runner;
    }

    /// Return the runner used for one consistent execution snapshot.
    /// 获取当前 runner 的共享快照；未配置钩子时返回 `None`。
    pub fn runner_snapshot(&self) -> Option<Arc<HookRunner>> {
        self.runner.read().unwrap_or_else(|error| error.into_inner()).clone()
    }

    /// Return the shared, bounded command audit store.
    /// 返回跨 runner 替换保持稳定的命令审计存储。
    pub fn audit_store(&self) -> CommandAuditStore {
        self.audit_store.clone()
    }
}

/// Build the shared hook runtime through the single supported assembly entry point.
/// 使用配置构建钩子运行时。
pub fn build_hook_runtime(config: &Config) -> Result<HookRuntime> {
    HookRuntime::build(config)
}

fn build_candidate(
    config: &Config,
    audit_store: CommandAuditStore,
) -> Result<HookRuntimeCandidate> {
    if !config.hooks.enabled {
        if !config.hooks.required.is_empty() {
            bail!("required hooks are disabled");
        }
        return Ok(HookRuntimeCandidate { runner: None });
    }

    let mut runner = HookRunner::new();
    let mut registered = std::collections::HashSet::new();
    if config.hooks.builtin.command_logger {
        runner.register(Box::new(CommandLoggerHook::with_store(audit_store)));
    }
    for handler in crate::plugins::trusted_plugin_hook_handlers(config)
        .context("failed to build trusted plugin hooks")?
    {
        registered.insert(handler.name().to_owned());
        runner.register(handler);
    }
    for required in &config.hooks.required {
        if !registered.contains(required) {
            bail!("required hook is unavailable: {required}");
        }
    }

    Ok(HookRuntimeCandidate { runner: Some(Arc::new(runner)) })
}

/// Build execution gates, retaining process-pinned requirements across config edits.
pub(crate) fn execution_hook_runner(config: &Config) -> Result<Option<Arc<HookRunner>>> {
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
    Ok(HookRuntime::build(&config)?.runner_snapshot())
}

/// Check model admission immediately before a provider request, without exposing prompts.
pub(crate) async fn authorize_model_request(config: &Config, model: &str) -> Result<bool> {
    if let Some(runner) = execution_hook_runner(config)? {
        match runner.run_before_llm_call(Vec::new(), model.to_owned()).await {
            super::HookResult::Cancel(reason) => bail!("model request denied by hook: {reason}"),
            super::HookResult::Continue((messages, selected)) => {
                if !messages.is_empty() || selected != model {
                    bail!("model admission hooks cannot rewrite the request");
                }
            }
        }
    }
    Ok(!config.hooks.required.is_empty()
        || std::env::var_os("VW_REQUIRED_EXECUTION_HOOKS").is_some())
}
