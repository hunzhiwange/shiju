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
use crate::app::agent::config::schema::LEGACY_CONFIG_JSON_FILENAME;
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
    vibewindow_dir.join(LEGACY_CONFIG_JSON_FILENAME)
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
pub(super) async fn load_config_for_tenant_from_store(
    store: &SystemConfigStore,
    tenant: &TenantScope,
    project_root: Option<&Path>,
    vibewindow_dir: &Path,
    workspace_dir: PathBuf,
    resolution_source: ConfigResolutionSource,
    apply_environment: impl FnOnce(&mut Config),
) -> Result<Config> {
    let config_path = config_json_path(vibewindow_dir);
    let project_key = project_root
        .map(|root| root.to_str().map(str::to_owned).context("Project root must be valid UTF-8"))
        .transpose()?;
    let blocking_store = store.clone();
    let blocking_tenant = tenant.clone();
    let (stored_payload, project_payload) = tokio::task::spawn_blocking(move || {
        let system_payload = blocking_store.get(&blocking_tenant)?;
        let project_payload = project_key
            .as_deref()
            .map(|key| blocking_store.get_project(&blocking_tenant, key))
            .transpose()?
            .flatten();
        Ok::<_, anyhow::Error>((system_payload, project_payload))
    })
    .await
    .context("join system config load task")??;
    let (base_payload_json, initialized) = match stored_payload {
        Some(payload) => (payload, false),
        None => {
            let mut config = Config::default();
            config.config_path = config_json_path(vibewindow_dir);
            config.workspace_dir = workspace_dir.clone();
            validate_config(&config)?;
            encrypt_config_secrets(&mut config)?;
            let payload =
                serde_json::to_string(&config).context("serialize default tenant system config")?;
            (payload, true)
        }
    };

    let mut merged_payload: Value = serde_json::from_str(&base_payload_json)
        .context("Failed to parse tenant system config payload")?;
    if !merged_payload.is_object() {
        bail!("tenant system config payload root must be an object");
    }

    if let Some(project_payload) = project_payload {
        let project_override: Value = serde_json::from_str(&project_payload)
            .context("Failed to parse tenant project config payload")?;
        if !project_override.is_object() {
            bail!("tenant project config payload root must be an object");
        }
        merge_config_payload(&mut merged_payload, project_override);
    }

    let config = decode_json_config_with_environment(
        merged_payload,
        &config_path,
        workspace_dir,
        vibewindow_dir,
        apply_environment,
    )?;
    if initialized {
        let blocking_store = store.clone();
        let blocking_tenant = tenant.clone();
        let blocking_payload = base_payload_json.clone();
        tokio::task::spawn_blocking(move || {
            blocking_store.upsert(&blocking_tenant, &blocking_payload)
        })
        .await
        .context("join system config initialization task")??;
    }
    log_loaded_config(&config, resolution_source, initialized);
    Ok(config)
}

/// 按显式租户加载数据库基础配置，并可选择叠加项目路径对应的数据库覆盖。
///
/// 本函数不会向项目父目录搜索。旧用户配置迁移仅在加载 `local/local` 时执行。
#[cfg(not(target_arch = "wasm32"))]
async fn load_or_init_config_for_tenant_with_environment(
    tenant: &TenantScope,
    project_root: Option<&Path>,
    apply_environment: impl FnOnce(&mut Config),
) -> Result<Config> {
    let home = UserDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .context("Could not find home directory")?;
    if tenant == &TenantScope::default() {
        migrate_user_config(&home)?;
    }

    let (default_vibewindow_dir, default_workspace_dir) = default_config_and_workspace_dirs()?;
    let (vibewindow_dir, resolved_workspace_dir, resolution_source) =
        resolve_runtime_config_dirs(&default_vibewindow_dir, &default_workspace_dir).await?;
    fs::create_dir_all(&vibewindow_dir)
        .await
        .with_context(|| config_dir_creation_error(&vibewindow_dir))?;

    let workspace_dir = project_root.map(Path::to_path_buf).unwrap_or(resolved_workspace_dir);
    if project_root.is_none() {
        fs::create_dir_all(&workspace_dir).await.context("Failed to create workspace directory")?;
    }

    #[cfg(test)]
    let store = SystemConfigStore::new(GlobalDatabase::for_home(home)?)?;
    #[cfg(not(test))]
    let store = {
        let database = GlobalDatabase::for_home(home)?;
        tokio::task::spawn_blocking(move || SystemConfigStore::new(database))
    }
    .await
    .context("join system config store initialization task")??;
    load_config_for_tenant_from_store(
        &store,
        tenant,
        project_root,
        &vibewindow_dir,
        workspace_dir,
        resolution_source,
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
