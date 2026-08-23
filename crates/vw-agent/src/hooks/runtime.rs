//! 可热替换的钩子运行时，保证候选构建完成后再原子发布。
//!
//! 命令审计存储跨替换保持稳定；候选构建失败不会影响当前正在服务的 runner 快照。

use std::sync::{Arc, RwLock};

use anyhow::{Context, Result};

use crate::app::agent::config::Config;

use super::builtin::{CommandAuditStore, CommandLoggerHook};
use super::HookRunner;

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

fn build_candidate(config: &Config, audit_store: CommandAuditStore) -> Result<HookRuntimeCandidate> {
    if !config.hooks.enabled {
        return Ok(HookRuntimeCandidate { runner: None });
    }

    let mut runner = HookRunner::new();
    if config.hooks.builtin.command_logger {
        runner.register(Box::new(CommandLoggerHook::with_store(audit_store)));
    }
    for handler in crate::plugins::trusted_plugin_hook_handlers(config)
        .context("failed to build trusted plugin hooks")?
    {
        runner.register(handler);
    }

    Ok(HookRuntimeCandidate { runner: Some(Arc::new(runner)) })
}
