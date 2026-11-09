//! 配置加载与首次初始化流程。
//!
//! 本模块负责迁移旧用户配置、加载租户数据库基础 payload、叠加显式项目覆盖，并在统一反序列化后
//! 解密密钥、展开环境变量引用、应用环境变量和执行最终校验。加载失败会显式返回错误，避免 agent 在
//! 部分配置状态下启动。

use anyhow::{Context, Result, bail};
#[cfg(not(target_arch = "wasm32"))]
use directories::UserDirs;
use serde_json::Value;
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use tokio::fs;

use vw_api_types::id::TenantScope;
use vw_config_types::config::Config;

#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::config::SystemConfigStore;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::config::migration::migrate_user_config;
use crate::app::agent::config::schema::config_env::apply_env_overrides;
use crate::app::agent::config::schema::config_helpers::config_dir_creation_error;
use crate::app::agent::config::schema::config_io::{
    extract_config_payload, merge_config_payload, normalize_legacy_alias_conflicts,
    normalize_top_level_table_aliases, read_config_root_json,
};
use crate::app::agent::config::schema::config_secrets::{
    decrypt_config_secrets, encrypt_config_secrets,
};
use crate::app::agent::config::schema::config_validate::validate_config;
use crate::app::agent::config::schema::resolve_telegram_allowed_users_env_refs;
use crate::app::agent::config::schema::workspace::ConfigResolutionSource;
use crate::app::agent::config::schema::{
    default_config_and_workspace_dirs, resolve_runtime_config_dirs,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::global_database::GlobalDatabase;

pub(super) fn is_retired_agent_tool_policy(path: &str) -> bool {
    let mut segments = path.split('.');
    matches!(segments.next(), Some("agents"))
        && segments.next().is_some()
        && matches!(segments.next(), Some("tool_policy"))
        && segments.next().is_none()
}

fn log_ignored_config_key(path: &str) {
    if is_retired_agent_tool_policy(path) {
        tracing::debug!(key = %path, "Retired config key ignored");
    } else {
        tracing::warn!(key = %path, "Unknown config key ignored");
    }
}

/// 返回当前配置目录下的 JSON 配置文件路径。
///
/// 参数 `vibewindow_dir` 是已经解析好的 VibeWindow 配置目录；函数只拼接文件名，不访问文件系统。
pub(crate) fn config_json_path(vibewindow_dir: &Path) -> PathBuf {
    vibewindow_dir.join(vw_config_types::paths::CONFIG_JSON_FILENAME)
}

/// 从旧 TOML 配置文件加载完整配置。
///
/// `config_path` 指向 TOML 文件，`workspace_dir` 和 `vibewindow_dir` 分别提供运行时 workspace 与密钥目录。
/// 加载过程会归一化历史表名、记录未知键、解密密钥、解析 Telegram 环境变量引用、应用环境变量覆盖并校验配置。
///
/// WASM 目标不支持本地 TOML 文件读取，会直接返回不支持错误；桌面/服务端目标上的读取、解析、解密或校验失败
/// 都会返回带上下文的错误。
pub(crate) async fn load_config_from_toml_path(
    config_path: &Path,
    workspace_dir: PathBuf,
    vibewindow_dir: &Path,
    resolution_source: ConfigResolutionSource,
) -> Result<Config> {
    #[cfg(target_arch = "wasm32")]
    anyhow::bail!("Not supported in WASM");

    #[cfg(not(target_arch = "wasm32"))]
    {
        let contents =
            fs::read_to_string(config_path).await.context("Failed to read config file")?;
        let mut raw_toml: toml::Value =
            toml::from_str(&contents).context("Failed to parse config file")?;
        normalize_top_level_table_aliases(&mut raw_toml);
        let normalized_contents =
            toml::to_string(&raw_toml).context("Failed to normalize config file")?;

        let mut ignored_paths: Vec<String> = Vec::new();
        let deserializer = toml::Deserializer::parse(&normalized_contents)
            .context("Failed to prepare normalized config")?;
        // 使用 serde_ignored 保持向后兼容：未知键不阻断启动，但必须记录，方便用户发现拼写错误。
        let mut config: Config = serde_ignored::deserialize(deserializer, |path| {
            ignored_paths.push(path.to_string());
        })
        .context("Failed to deserialize config file")?;

        for path in ignored_paths {
            log_ignored_config_key(&path);
        }
        config.config_path = config_path.to_path_buf();
        config.workspace_dir = workspace_dir;
        decrypt_config_secrets(&mut config, vibewindow_dir)?;
        resolve_telegram_allowed_users_env_refs(&mut config.channels_config)?;
        apply_env_overrides(&mut config);
        validate_config(&config)?;
        tracing::debug!(
            path = %config.config_path.display(),
            workspace = %config.workspace_dir.display(),
            source = resolution_source.as_str(),
            initialized = false,
            "Config loaded"
        );
        Ok(config)
    }
}

fn decode_json_config_with_environment(
    mut config_payload: Value,
    config_path: &Path,
    workspace_dir: PathBuf,
    vibewindow_dir: &Path,
    apply_environment: impl FnOnce(&mut Config),
) -> Result<Config> {
    if !config_payload.is_object() {
        bail!("config payload root must be an object");
    }

    normalize_legacy_alias_conflicts(&mut config_payload);
    let config_json =
        serde_json::to_string(&config_payload).context("Failed to serialize config payload")?;
    let mut ignored_paths = Vec::new();
    let mut deserializer = serde_json::Deserializer::from_str(&config_json);
    let mut config: Config = serde_ignored::deserialize(&mut deserializer, |path| {
        ignored_paths.push(path.to_string());
    })
    .context("Failed to deserialize config payload")?;

    for path in ignored_paths {
        log_ignored_config_key(&path);
    }
    config.config_path = config_path.to_path_buf();
    config.workspace_dir = workspace_dir;
    decrypt_config_secrets(&mut config, vibewindow_dir)?;
    resolve_telegram_allowed_users_env_refs(&mut config.channels_config)?;
    apply_environment(&mut config);
    validate_config(&config)?;
    Ok(config)
}

fn log_loaded_config(
    config: &Config,
    resolution_source: ConfigResolutionSource,
    initialized: bool,
) {
    tracing::debug!(
        path = %config.config_path.display(),
        workspace = %config.workspace_dir.display(),
        source = resolution_source.as_str(),
        initialized,
        "Config loaded"
    );
}

/// 从 JSON 根对象加载配置。
///
/// 返回 `Ok(None)` 表示配置文件不存在或为空，调用方可以继续初始化默认配置。返回 `Ok(Some(config))`
/// 表示已经完成反序列化、旧别名归一化、密钥解密、环境变量覆盖和配置校验。
///
/// WASM 目标不读取本地文件，因此始终返回 `Ok(None)`；非 WASM 目标上，读取、解析、解密或校验失败会返回错误。
pub(crate) async fn load_config_from_json_root(
    config_path: &Path,
    workspace_dir: PathBuf,
    vibewindow_dir: &Path,
    resolution_source: ConfigResolutionSource,
) -> Result<Option<Config>> {
    #[cfg(target_arch = "wasm32")]
    return Ok(None);

    #[cfg(not(target_arch = "wasm32"))]
    {
        let root = read_config_root_json(config_path).await?;
        let Some(config_payload) = extract_config_payload(&root) else {
            return Ok(None);
        };
        let config = decode_json_config_with_environment(
            config_payload,
            config_path,
            workspace_dir,
            vibewindow_dir,
            apply_env_overrides,
        )?;
        log_loaded_config(&config, resolution_source, false);
        Ok(Some(config))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn default_tenant_payload(config_path: &Path, workspace_dir: &Path) -> Result<String> {
    let mut config = Config::default();
    config.config_path = config_path.to_path_buf();
    config.workspace_dir = workspace_dir.to_path_buf();
    validate_config(&config)?;
    encrypt_config_secrets(&mut config)?;
    serde_json::to_string(&config).context("serialize default tenant system config")
}

#[cfg(not(target_arch = "wasm32"))]
fn decode_tenant_config(
    base_payload: &str,
    project_payload: Option<&str>,
    config_path: &Path,
    workspace_dir: PathBuf,
    vibewindow_dir: &Path,
    apply_environment: impl FnOnce(&mut Config),
) -> Result<Config> {
    let mut merged_payload: Value = serde_json::from_str(base_payload)
        .context("Failed to parse tenant system config payload")?;
    if !merged_payload.is_object() {
        bail!("tenant system config payload root must be an object");
    }

    if let Some(project_payload) = project_payload {
        let mut project_override: Value = serde_json::from_str(project_payload)
            .context("Failed to parse tenant project config payload")?;
        let Some(override_object) = project_override.as_object_mut() else {
            bail!("tenant project config payload root must be an object");
        };
        // Agent definitions belong to the tenant, never to a workspace.
        override_object.remove("agents");
        merge_config_payload(&mut merged_payload, project_override);
    }

    decode_json_config_with_environment(
        merged_payload,
        config_path,
        workspace_dir,
        vibewindow_dir,
        apply_environment,
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn load_config_for_tenant_from_store(
    store: &SystemConfigStore,
    tenant: &TenantScope,
    project_root: Option<&Path>,
    vibewindow_dir: &Path,
    workspace_dir: PathBuf,
    resolution_source: ConfigResolutionSource,
    apply_environment: impl Fn(&mut Config) + Send + 'static,
) -> Result<Config> {
    let vibewindow_dir = vibewindow_dir.to_path_buf();
    let project_key = project_root
        .map(|root| root.to_str().map(str::to_owned).context("Project root must be valid UTF-8"))
        .transpose()?;
    let store = store.clone();
    let tenant = tenant.clone();
    // Keep the synchronous config decode off the nested async request stack.
    // Box the result so creating the blocking task does not reserve a Config on that stack.
    let (config, initialized) = tokio::task::spawn_blocking(move || {
        let config_path = config_json_path(&vibewindow_dir);
        let stored_payload = store.get(&tenant)?;
        let project_payload = project_key
            .as_deref()
            .map(|key| store.get_project(&tenant, key))
            .transpose()?
            .flatten();
        let decode = |payload: &str| {
            decode_tenant_config(
                payload,
                project_payload.as_deref(),
                &config_path,
                workspace_dir.clone(),
                &vibewindow_dir,
                &apply_environment,
            )
            .map(Box::new)
        };
        if let Some(payload) = stored_payload {
            return Ok((decode(&payload)?, false));
        }

        let payload = default_tenant_payload(&config_path, &workspace_dir)?;
        let config = decode(&payload)?;
        if store.compare_and_swap(&tenant, None, &payload, None)? {
            return Ok((config, true));
        }
        let winner = store.get(&tenant)?.context("tenant system config disappeared")?;
        Ok::<_, anyhow::Error>((decode(&winner)?, false))
    })
    .await
    .context("join system config load task")??;
    log_loaded_config(&config, resolution_source, initialized);
    Ok(*config)
}

/// The same resolved store and paths are used for reads and optimistic updates.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct TenantConfigSource {
    pub(crate) store: SystemConfigStore,
    config_dir: PathBuf,
    workspace_dir: PathBuf,
    resolution_source: ConfigResolutionSource,
}

#[cfg(not(target_arch = "wasm32"))]
impl TenantConfigSource {
    pub(crate) async fn open(tenant: &TenantScope, project_root: Option<&Path>) -> Result<Self> {
        let home = UserDirs::new()
            .map(|dirs| dirs.home_dir().to_path_buf())
            .context("Could not find home directory")?;
        #[cfg(test)]
        let home =
            std::env::var_os("VW_MODEL_RECOVERY_TEST_ROOT").map(PathBuf::from).unwrap_or(home);

        let migrate = tenant == &TenantScope::default();
        let open_store = move || {
            if migrate {
                migrate_user_config(&home)?;
            }
            SystemConfigStore::new(GlobalDatabase::for_home(home)?)
        };
        #[cfg(test)]
        let open_store = crate::global_database::inherit_test_database_context(open_store);
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
    pub(crate) async fn read_for_patch(
        &self,
        tenant: &TenantScope,
    ) -> Result<(Config, Option<String>)> {
        let store = self.store.clone();
        let tenant = tenant.clone();
        let snapshot = tokio::task::spawn_blocking(move || store.get(&tenant))
            .await
            .context("join system config load task")??;
        let config_path = config_json_path(&self.config_dir);
        let default;
        let payload = match snapshot.as_deref() {
            Some(payload) => payload,
            None => {
                default = default_tenant_payload(&config_path, &self.workspace_dir)?;
                &default
            }
        };
        let config = decode_tenant_config(
            payload,
            None,
            &config_path,
            self.workspace_dir.clone(),
            &self.config_dir,
            apply_env_overrides,
        )?;
        Ok((config, snapshot))
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

pub(crate) async fn load_or_init_config_for_tenant_without_env(
    tenant: &TenantScope,
    project_root: Option<&Path>,
) -> Result<Config> {
    load_or_init_config_for_tenant_with_environment(tenant, project_root, |_| {}).await
}

/// 使用本地租户加载配置的兼容入口。
pub async fn load_or_init_config() -> Result<Config> {
    load_or_init_config_for_tenant(&TenantScope::default(), None).await
}
