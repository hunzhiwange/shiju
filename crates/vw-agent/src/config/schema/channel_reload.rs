//! 通道配置监听器的宿主组合：数据库初始化与当前租户捕获。
use super::{ChannelsConfig, Config};
use crate::config::SystemConfigStore;
use crate::global_database::GlobalDatabase;
use anyhow::{Context, Result};
use vw_api_transport::id::TenantScope;

impl agent_config::schema::channel_reload::ChannelConfigStore for SystemConfigStore {
    fn get(&self, tenant: &TenantScope) -> Result<Option<String>> { SystemConfigStore::get(self, tenant) }
    fn get_project(&self, tenant: &TenantScope, project: &str) -> Result<Option<String>> {
        SystemConfigStore::get_project(self, tenant, project)
    }
}

pub(crate) struct ChannelConfigWatcher {
    inner: agent_config::schema::channel_reload::ChannelConfigWatcher<SystemConfigStore>,
    #[cfg(test)]
    store: SystemConfigStore,
}
impl ChannelConfigWatcher {
    pub(crate) async fn new(config: &Config, include_project: bool) -> Result<Self> {
        let home = agent_config::runtime_home::config_home()?;
        let open_store = move || SystemConfigStore::new(GlobalDatabase::for_home(home)?);
        #[cfg(test)]
        let open_store = crate::global_database::inherit_test_database_context(open_store);
        let store = tokio::task::spawn_blocking(open_store)
            .await
            .context("join channel config store initialization")??;
        Self::from_store(store, config, include_project)
    }

    pub(crate) fn from_store(store: SystemConfigStore, config: &Config, include_project: bool) -> Result<Self> {
        let inner = agent_config::schema::channel_reload::ChannelConfigWatcher::from_store(
            store.clone(), config, include_project, crate::session::ui_store::current_tenant_scope(),
            super::apply_env_overrides, super::validate_config, super::config_io::merge_config_payload)?;
        Ok(Self { inner, #[cfg(test)] store })
    }
    pub(crate) async fn poll(&mut self, current: &Config) -> Result<Option<ChannelsConfig>> {
        self.inner.poll(current).await
    }
    pub(crate) async fn load_for_delivery(&mut self, current: &Config) -> Result<ChannelsConfig> {
        self.inner.load_for_delivery(current).await
    }
}
#[cfg(test)]
#[path = "channel_reload_tests.rs"]
mod tests;
