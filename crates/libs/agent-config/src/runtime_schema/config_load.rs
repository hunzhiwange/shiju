//! 配置加载与首次初始化流程。
//!
//! 本模块负责迁移旧用户配置、加载租户数据库基础 payload、叠加显式项目覆盖，并在统一反序列化后
//! 解密密钥、展开环境变量引用、应用环境变量和执行最终校验。加载失败会显式返回错误，避免 agent 在
//! 部分配置状态下启动。

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use tokio::fs;

use vw_api_transport::id::TenantScope;
use config_aggregate::Config;

#[cfg(not(target_arch = "wasm32"))]
use crate::store::SystemConfigStore;
#[cfg(not(target_arch = "wasm32"))]
use crate::runtime_migration::migrate_user_config;
use crate::runtime_schema::config_env::apply_env_overrides;
use crate::runtime_schema::config_helpers::config_dir_creation_error;
#[cfg(target_arch = "wasm32")]
use crate::runtime_schema::config_validate::validate_config;
use crate::schema::workspace::ConfigResolutionSource;
use crate::runtime_schema::{
    default_config_and_workspace_dirs, resolve_runtime_config_dirs,
};
#[cfg(not(target_arch = "wasm32"))]
use agent_database::GlobalDatabase;

pub use crate::schema::config_load::{config_json_path, is_retired_agent_tool_policy};

pub async fn load_config_from_toml_path(config_path: &Path, workspace_dir: PathBuf, config_dir: &Path, source: ConfigResolutionSource) -> Result<Config> {
    crate::schema::config_load::load_config_from_toml_path(config_path, workspace_dir, config_dir, source.as_str(), apply_env_overrides).await
}
pub async fn load_config_from_json_root(config_path: &Path, workspace_dir: PathBuf, config_dir: &Path, source: ConfigResolutionSource) -> Result<Option<Config>> {
    crate::schema::config_load::load_config_from_json_root(config_path, workspace_dir, config_dir, source.as_str(), apply_env_overrides).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn load_config_for_tenant_from_store(store: &SystemConfigStore, tenant: &TenantScope, project_root: Option<&Path>, config_dir: &Path, workspace_dir: PathBuf, source: ConfigResolutionSource, apply_environment: impl Fn(&mut Config) + Send + 'static) -> Result<Config> {
    crate::schema::config_load::load_config_for_tenant_from_store(store, tenant, project_root, config_dir, workspace_dir, source.as_str(), apply_environment).await
}
#[cfg(not(target_arch = "wasm32"))]
impl crate::schema::config_load::ConfigLoadStore for SystemConfigStore {
    fn get(&self, tenant: &TenantScope) -> Result<Option<String>> { self.get(tenant) }
    fn get_project(&self, tenant: &TenantScope, project: &str) -> Result<Option<String>> { self.get_project(tenant, project) }
    fn initialize_if_absent(&self, tenant: &TenantScope, payload: &str) -> Result<bool> { self.compare_and_swap(tenant, None, payload, None) }
}

/// The same resolved store and paths are used for reads and optimistic updates.
#[cfg(not(target_arch = "wasm32"))]
pub struct TenantConfigSource {
    pub store: SystemConfigStore,
    config_dir: PathBuf,
    workspace_dir: PathBuf,
    resolution_source: ConfigResolutionSource,
}

#[cfg(not(target_arch = "wasm32"))]
impl TenantConfigSource {
    pub async fn open(tenant: &TenantScope, project_root: Option<&Path>) -> Result<Self> {
        let home = crate::runtime_home::config_home()?;
        #[cfg(any(test, feature = "test-support"))]
        let home =
            std::env::var_os("VW_MODEL_RECOVERY_TEST_ROOT").map(PathBuf::from).unwrap_or(home);

        let migrate = tenant == &TenantScope::default();
        let open_store = move || {
            if migrate {
                migrate_user_config(&home)?;
            }
            SystemConfigStore::new(GlobalDatabase::for_home(home)?)
        };
        #[cfg(any(test, feature = "test-support"))]
        let open_store = agent_database::inherit_test_database_context(open_store);
        let store = tokio::task::spawn_blocking(open_store)
            .await
            .context("join system config store initialization task")??;

        let (default_config_dir, default_workspace_dir) = default_config_and_workspace_dirs()?;
        let (config_dir, resolved_workspace_dir, resolution_source) =
            resolve_runtime_config_dirs(&default_config_dir, &default_workspace_dir).await?;
        fs::create_dir_all(&config_dir)
            .await
            .with_context(|| config_dir_creation_error(&config_dir))?;

        let workspace_dir = project_root.map(Path::to_path_buf).unwrap_or(resolved_workspace_dir);
        if project_root.is_none() {
            fs::create_dir_all(&workspace_dir)
                .await
                .context("Failed to create workspace directory")?;
        }
        Ok(Self { store, config_dir, workspace_dir, resolution_source })
    }

    /// Load the base and its exact persisted snapshot without initializing an absent row.
    pub async fn read_for_patch(
        &self,
        tenant: &TenantScope,
    ) -> Result<(Config, Option<String>)> {
        crate::schema::config_load::read_for_patch(&self.store, tenant, &self.config_dir, &self.workspace_dir, apply_env_overrides).await
    }
}

/// 按显式租户加载数据库基础配置，并可选择叠加项目路径对应的数据库覆盖。
///
/// 本函数不会向项目父目录搜索。旧用户配置迁移仅在加载 `local/local` 时执行。
#[cfg(not(target_arch = "wasm32"))]
async fn load_or_init_config_for_tenant_with_environment(
    tenant: &TenantScope,
    project_root: Option<&Path>,
    apply_environment: impl Fn(&mut Config) + Send + 'static,
) -> Result<Config> {
    let source = TenantConfigSource::open(tenant, project_root).await?;
    load_config_for_tenant_from_store(
        &source.store,
        tenant,
        project_root,
        &source.config_dir,
        source.workspace_dir,
        source.resolution_source,
        apply_environment,
    )
    .await
}

#[cfg(target_arch = "wasm32")]
async fn load_or_init_config_for_tenant_with_environment(
    _tenant: &TenantScope,
    _project_root: Option<&Path>,
    apply_environment: impl FnOnce(&mut Config),
) -> Result<Config> {
    let mut config = Config::default();
    apply_environment(&mut config);
    validate_config(&config)?;
    Ok(config)
}

pub async fn load_or_init_config_for_tenant(
    tenant: &TenantScope,
    project_root: Option<&Path>,
) -> Result<Config> {
    load_or_init_config_for_tenant_with_environment(tenant, project_root, apply_env_overrides).await
}

pub async fn load_or_init_config_for_tenant_without_env(
    tenant: &TenantScope,
    project_root: Option<&Path>,
) -> Result<Config> {
    load_or_init_config_for_tenant_with_environment(tenant, project_root, |_| {}).await
}

/// 使用本地租户加载配置的兼容入口。
pub async fn load_or_init_config() -> Result<Config> {
    load_or_init_config_for_tenant(&TenantScope::default(), None).await
}
