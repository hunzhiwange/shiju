//! 租户配置保存流程，负责校验、密钥加密及全局与项目覆盖的持久化。
//!
//! 持久化前会移除运行期 skey；项目配置只保存相对基础配置的覆盖形状，避免复制全局值。

use anyhow::{Context, Result};
#[cfg(target_arch = "wasm32")]
use anyhow::bail;
use serde_json::Value;
use std::path::Path;
use vw_api_transport::id::TenantScope;
use config_aggregate::Config;

#[cfg(not(target_arch = "wasm32"))]
use crate::store::SystemConfigStore;
#[cfg(not(target_arch = "wasm32"))]
use crate::runtime_schema::config_load::load_or_init_config_for_tenant;
#[cfg(not(target_arch = "wasm32"))]
use agent_database::GlobalDatabase;

#[cfg(not(target_arch = "wasm32"))]
/// 校验并加密配置后写入指定租户；运行期 skey 永不落库。
pub fn save_config_to_store(
    config: &Config,
    tenant: &TenantScope,
    store: &SystemConfigStore,
) -> Result<()> {
    store.upsert(tenant, &encode_config_for_storage(config)?)
}

/// 将配置保存到指定租户；原生平台把阻塞式数据库操作移出异步执行器。
pub async fn save_config_for_tenant(config: &Config, tenant: &TenantScope) -> Result<()> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (config, tenant);
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let home = crate::runtime_home::config_home()?;
        #[cfg(any(test, feature = "test-support"))]
        {
            let store = SystemConfigStore::new(GlobalDatabase::for_home(home)?)?;
            return save_config_to_store(config, tenant, &store);
        }

        #[cfg(not(any(test, feature = "test-support")))]
        let config = config.clone();
        #[cfg(not(any(test, feature = "test-support")))]
        let tenant = tenant.clone();
        #[cfg(not(any(test, feature = "test-support")))]
        tokio::task::spawn_blocking(move || {
            let store = SystemConfigStore::new(GlobalDatabase::for_home(home)?)?;
            save_config_to_store(&config, &tenant, &store)
        })
        .await
        .context("join system config save task")?
    }
}

/// 将配置保存到默认租户。
pub async fn save_config(config: &Config) -> Result<()> {
    save_config_for_tenant(config, &TenantScope::default()).await
}

#[cfg(not(target_arch = "wasm32"))]
pub use crate::schema::config_save::save_project_config_to_store;

#[cfg(not(target_arch = "wasm32"))]
async fn open_system_config_store() -> Result<SystemConfigStore> {
    let home = crate::runtime_home::config_home()?;
    let database = GlobalDatabase::for_home(home)?;
    tokio::task::spawn_blocking(move || SystemConfigStore::new(database))
        .await
        .context("join system config store initialization task")?
}

pub async fn update_project_config_for_tenant(tenant: &TenantScope, project: &Path, patch: Value) -> Result<Config> {
    #[cfg(not(target_arch = "wasm32"))]
    { crate::schema::config_save::update_project_config_for_tenant::<ProjectConfigHost>(tenant, project, patch).await }
    #[cfg(target_arch = "wasm32")]
    { let _ = (tenant, project, patch); bail!("Project config overrides are not supported in WASM"); }
}
pub async fn update_project_config_from_configs(project: &Path, before: &Config, after: &Config) -> Result<Config> {
    #[cfg(not(target_arch = "wasm32"))]
    { crate::schema::config_save::update_project_config_from_configs::<ProjectConfigHost>(project, before, after).await }
    #[cfg(target_arch = "wasm32")]
    { let before_value = serde_json::to_value(before).context("Failed to serialize previous config")?; let after_value = serde_json::to_value(after).context("Failed to serialize updated config")?; if merge_patch_between(&before_value, &after_value).is_none() { return Ok(after.clone()); } let _ = project; bail!("Project config overrides are not supported in WASM"); }
}
#[cfg(not(target_arch = "wasm32"))]
pub use crate::schema::config_save::encode_config_for_storage;
#[cfg(any(test, feature = "test-support"))]
pub use crate::schema::config_save::{merge_raw_project_patch, select_override_shape};
#[cfg(any(test, target_arch = "wasm32"))]
pub use crate::schema::config_save::merge_patch_between;

#[cfg(not(target_arch = "wasm32"))]
pub fn prepare_project_override(base: &Config, raw_override: Value, patch: Value) -> Result<Value> {
    let catalog = crate::catalog::merged_agent_catalog(&base.agents);
    crate::schema::config_save::prepare_project_override(base, raw_override, patch, &catalog)
}

#[cfg(not(target_arch = "wasm32"))]
/// 项目配置持久化的宿主组合。
struct ProjectConfigHost;
#[cfg(not(target_arch = "wasm32"))]
impl crate::schema::config_save::ProjectConfigHost for ProjectConfigHost {
    type Store = SystemConfigStore;
    async fn open_store() -> Result<Self::Store> { open_system_config_store().await }
    async fn load(tenant: &TenantScope, project: Option<&Path>) -> Result<Config> { load_or_init_config_for_tenant(tenant, project).await }
    fn catalog(config: &Config) -> std::collections::HashMap<String, config_agent::AgentDefinitionConfig> { crate::catalog::merged_agent_catalog(&config.agents) }
}
#[cfg(not(target_arch = "wasm32"))]
impl crate::schema::config_save::ProjectConfigStore for SystemConfigStore {
    fn get_project(&self, tenant: &TenantScope, project: &str) -> Result<Option<String>> { self.get_project(tenant, project) }
    fn upsert_project(&self, tenant: &TenantScope, project: &str, payload: &str) -> Result<()> { self.upsert_project(tenant, project, payload) }
}
