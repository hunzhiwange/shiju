//! Immutable per-turn plugin selection and cooperative package lifecycle leases.
use crate::plugins::{PluginDiscovery, PluginDiscoveryOptions, discover_plugins};
use crate::{config::Config, hooks::HookRunner};
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
pub(crate) struct PluginSessionSnapshot {
    options: PluginDiscoveryOptions,
    discovery: PluginDiscovery,
    tenant: vw_api_types::id::TenantScope,
    active: AtomicBool,
    pub(crate) hooks: Option<Arc<HookRunner>>,
    pub(crate) required_hooks: bool,
}
/// Only the request stack owns leases. Drop joins cancelled asynchronous hooks first.
pub(crate) struct PluginTurnSnapshot {
    pub(crate) data: Arc<PluginSessionSnapshot>,
    _leases: Vec<File>,
    async_hooks: Option<Arc<super::hook_async::AsyncHooks>>,
}
fn normalized(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}
impl PluginSessionSnapshot {
    pub(crate) fn validate_active(&self) -> Result<()> {
        ensure!(self.active.load(Ordering::Acquire), "plugin turn snapshot has expired");
        ensure!(
            self.tenant == crate::session::ui_store::current_tenant_scope(),
            "plugin snapshot tenant mismatch"
        );
        Ok(())
    }
    fn selection(
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
    pub(crate) fn capture(config: &Config) -> Result<Self> {
        let options = super::super::mcp::plugin_mcp_discovery_options(config);
        let mut roots = super::super::loader::plugin_source_dirs(&options)?;
        roots.sort();
        roots.dedup();
        let mut leases = Vec::new();
        for root in roots {
            if let Some(lease) = super::installed_state::read_lock(&root)? {
                leases.push(lease);
            }
        }
        let discovery = discover_plugins(&options)?;
        let hooks = crate::hooks::execution_hook_runner(config)?;
        Ok(Self {
            data: Arc::new(PluginSessionSnapshot {
                options,
                discovery,
                tenant: crate::session::ui_store::current_tenant_scope(),
                active: AtomicBool::new(true),
                hooks,
                required_hooks: !config.hooks.required.is_empty()
                    || std::env::var_os("VW_REQUIRED_EXECUTION_HOOKS").is_some(),
            }),
            _leases: leases,
            async_hooks: None,
        })
    }
    pub(crate) fn own_async_hooks(&mut self, hooks: Arc<super::hook_async::AsyncHooks>) {
        self.async_hooks = Some(hooks);
    }
}
impl Drop for PluginTurnSnapshot {
    fn drop(&mut self) {
        self.data.active.store(false, Ordering::Release);
        if let Some(hooks) = &self.async_hooks {
            crate::session::prompt::block_on(hooks.shutdown());
        }
        // Files are dropped after this method, so cancelled work cannot race publication.
    }
}
pub(crate) fn selection_for_runtime(
    options: &PluginDiscoveryOptions,
) -> Result<(PluginDiscovery, PluginDiscoveryOptions)> {
    if let Some(context) = crate::tools::current_tool_use_context()
        && let Some(snapshot) = context.plugin_snapshot()
    {
        return snapshot.selection(options);
    }
    Ok((discover_plugins(options)?, options.clone()))
}
pub(crate) fn discover_for_runtime(options: &PluginDiscoveryOptions) -> Result<PluginDiscovery> {
    selection_for_runtime(options).map(|(discovery, _)| discovery)
}
