//! 可热替换的钩子运行时，保证候选构建完成后再原子发布。

use std::sync::{Arc, RwLock};

use anyhow::{Context, Result};

use crate::app::agent::config::Config;

use super::builtin::{CommandAuditStore, CommandLoggerHook};
use super::HookRunner;

/// A fully built hook runner that can replace the active runner atomically.
pub struct HookRuntimeCandidate {
    runner: Option<Arc<HookRunner>>,
}

/// Shared hook runtime with stable audit storage and replaceable runner snapshots.
#[derive(Clone)]
pub struct HookRuntime {
    runner: Arc<RwLock<Option<Arc<HookRunner>>>>,
    audit_store: CommandAuditStore,
}

impl HookRuntime {
    /// Build the initial runtime without publishing a partially assembled runner.
    pub fn build(config: &Config) -> Result<Self> {
        let audit_store = CommandAuditStore::default();
        let candidate = build_candidate(config, audit_store.clone())?;
        Ok(Self { runner: Arc::new(RwLock::new(candidate.runner)), audit_store })
    }

    /// Build a replacement candidate while the current runner remains active.
    pub fn build_candidate(&self, config: &Config) -> Result<HookRuntimeCandidate> {
        build_candidate(config, self.audit_store.clone())
    }

    /// Publish a completely built candidate in one lock-protected replacement.
    pub fn replace(&self, candidate: HookRuntimeCandidate) {
        *self.runner.write().unwrap_or_else(|error| error.into_inner()) = candidate.runner;
    }

    /// Return the runner used for one consistent execution snapshot.
    pub fn runner_snapshot(&self) -> Option<Arc<HookRunner>> {
        self.runner.read().unwrap_or_else(|error| error.into_inner()).clone()
    }

    /// Return the shared, bounded command audit store.
    pub fn audit_store(&self) -> CommandAuditStore {
        self.audit_store.clone()
    }
}

/// Build the shared hook runtime through the single supported assembly entry point.
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
