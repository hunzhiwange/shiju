//! Immutable per-turn plugin selection and cooperative package lifecycle leases.
use crate::{PluginDiscovery, PluginDiscoveryOptions, discover_plugins};
use agent_hooks::HookRunner;
use anyhow::{Result, ensure};
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

/// Data may be retained by MCP clients; it must never own lifecycle locks.
pub struct PluginSessionSnapshot {
    options: PluginDiscoveryOptions,
    discovery: PluginDiscovery,
    tenant: vw_api_transport::TenantScope,
    active: AtomicBool,
    current_tenant: fn() -> vw_api_transport::TenantScope,
    pub hooks: Option<Arc<HookRunner>>,
    pub required_hooks: bool,
}
/// Only the request stack owns leases. Drop joins cancelled asynchronous hooks first.
pub struct PluginTurnSnapshot {
    pub data: Arc<PluginSessionSnapshot>,
    _leases: Vec<File>,
    async_hooks: Option<Arc<plugin_hooks::hook_async::AsyncHooks>>,
    shutdown: fn(&plugin_hooks::hook_async::AsyncHooks),
}
fn normalized(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}
impl PluginSessionSnapshot {
    pub fn validate_active(&self) -> Result<()> {
        ensure!(self.active.load(Ordering::Acquire), "plugin turn snapshot has expired");
        ensure!(
            self.tenant == (self.current_tenant)(),
            "plugin snapshot tenant mismatch"
        );
        Ok(())
    }
    pub fn selection(
        &self,
        options: &PluginDiscoveryOptions,
    ) -> Result<(PluginDiscovery, PluginDiscoveryOptions)> {
        self.validate_active()?;
        ensure!(
            normalized(&self.options.workspace_dir) == normalized(&options.workspace_dir)
                && normalized(&self.options.config_dir) == normalized(&options.config_dir),
            "plugin snapshot scope mismatch"
        );
        Ok((self.discovery.clone(), self.options.clone()))
    }
}
impl PluginTurnSnapshot {
    pub fn capture(
        options: PluginDiscoveryOptions,
        build_hooks: impl FnOnce() -> Result<Option<Arc<HookRunner>>>,
        required_hooks: impl FnOnce() -> bool,
        current_tenant: fn() -> vw_api_transport::TenantScope,
        shutdown: fn(&plugin_hooks::hook_async::AsyncHooks),
    ) -> Result<Self> {
        let mut roots = crate::loader::plugin_source_dirs(&options)?;
        roots.sort();
        roots.dedup();
        let mut leases = Vec::new();
        for root in roots {
            if let Some(lease) = plugin_install::installed_state::read_lock(&root, crate::load_plugin_manifest)? {
                leases.push(lease);
            }
        }
        let discovery = discover_plugins(&options)?;
        let hooks = build_hooks()?;
        Ok(Self {
            data: Arc::new(PluginSessionSnapshot {
                options,
                discovery,
                tenant: current_tenant(),
                current_tenant,
                active: AtomicBool::new(true),
                hooks,
                required_hooks: required_hooks(),
            }),
            _leases: leases,
            async_hooks: None,
            shutdown,
        })
    }
    pub fn own_async_hooks(&mut self, hooks: Arc<plugin_hooks::hook_async::AsyncHooks>) {
        self.async_hooks = Some(hooks);
    }
}
impl Drop for PluginTurnSnapshot {
    fn drop(&mut self) {
        self.data.active.store(false, Ordering::Release);
        if let Some(hooks) = &self.async_hooks {
            (self.shutdown)(hooks);
        }
        // Files are dropped after this method, so cancelled work cannot race publication.
    }
}
