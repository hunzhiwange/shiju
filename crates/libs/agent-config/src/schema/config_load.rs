//! 配置解码与租户加载；数据库后端由调用方注入。
use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::path::{Path, PathBuf};
use config_aggregate::Config;
use vw_api_transport::id::TenantScope;
#[cfg(not(target_arch = "wasm32"))]
use tokio::fs;
use super::config_io::{extract_config_payload, merge_config_payload, normalize_legacy_alias_conflicts, normalize_top_level_table_aliases, read_config_root_json};
use super::config_secrets::{decrypt_config_secrets, encrypt_config_secrets};
use super::config_validate::validate_config;
use super::resolve_telegram_allowed_users_env_refs;

/// 配置加载实际使用的存储操作；初始化必须维持原子竞争语义。
#[cfg(not(target_arch = "wasm32"))]
pub trait ConfigLoadStore: Clone + Send + 'static {
    fn get(&self, tenant: &TenantScope) -> Result<Option<String>>;
    fn get_project(&self, tenant: &TenantScope, project: &str) -> Result<Option<String>>;
    fn initialize_if_absent(&self, tenant: &TenantScope, payload: &str) -> Result<bool>;
}
pub fn is_retired_agent_tool_policy(path: &str) -> bool {
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
pub fn config_json_path(vibewindow_dir: &Path) -> PathBuf {
    vibewindow_dir.join(config_runtime_paths::CONFIG_JSON_FILENAME)
}

/// 从旧 TOML 配置文件加载完整配置。
///
/// `config_path` 指向 TOML 文件，`workspace_dir` 和 `vibewindow_dir` 分别提供运行时 workspace 与密钥目录。
/// 加载过程会归一化历史表名、记录未知键、解密密钥、解析 Telegram 环境变量引用、应用环境变量覆盖并校验配置。
///
/// WASM 目标不支持本地 TOML 文件读取，会直接返回不支持错误；桌面/服务端目标上的读取、解析、解密或校验失败
/// 都会返回带上下文的错误。
pub async fn load_config_from_toml_path(
    config_path: &Path,
    workspace_dir: PathBuf,
    vibewindow_dir: &Path,
    resolution_source: &str,
    apply_env_overrides: fn(&mut Config),
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
            source = resolution_source,
            initialized = false,
            "Config loaded"
        );
        Ok(config)
    }
}

pub fn decode_json_config_with_environment(
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
    resolution_source: &str,
    initialized: bool,
) {
    tracing::debug!(
        path = %config.config_path.display(),
        workspace = %config.workspace_dir.display(),
        source = resolution_source,
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
pub async fn load_config_from_json_root(
    config_path: &Path,
    workspace_dir: PathBuf,
    vibewindow_dir: &Path,
    resolution_source: &str,
    apply_env_overrides: fn(&mut Config),
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
pub fn default_tenant_payload(config_path: &Path, workspace_dir: &Path) -> Result<String> {
    let mut config = Config::default();
    config.config_path = config_path.to_path_buf();
    config.workspace_dir = workspace_dir.to_path_buf();
    validate_config(&config)?;
    encrypt_config_secrets(&mut config)?;
    serde_json::to_string(&config).context("serialize default tenant system config")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn decode_tenant_config(
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
        override_object.remove("worktree_settings");
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
pub async fn load_config_for_tenant_from_store<S: ConfigLoadStore>(
    store: &S,
    tenant: &TenantScope,
    project_root: Option<&Path>,
    vibewindow_dir: &Path,
    workspace_dir: PathBuf,
    resolution_source: &str,
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
        if store.initialize_if_absent(&tenant, &payload)? {
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


#[cfg(not(target_arch = "wasm32"))]
pub async fn read_for_patch<S: ConfigLoadStore>(store: &S, tenant: &TenantScope, config_dir: &Path, workspace_dir: &Path, apply_env_overrides: fn(&mut Config)) -> Result<(Config, Option<String>)> {
        let store = store.clone();
        let tenant = tenant.clone();
        let snapshot = tokio::task::spawn_blocking(move || store.get(&tenant))
            .await
            .context("join system config load task")??;
        let config_path = config_json_path(config_dir);
        let default;
        let payload = match snapshot.as_deref() {
            Some(payload) => payload,
            None => {
                default = default_tenant_payload(&config_path, workspace_dir)?;
                &default
            }
        };
        let config = decode_tenant_config(
            payload,
            None,
            &config_path,
            workspace_dir.to_path_buf(),
            config_dir,
            apply_env_overrides,
        )?;
        Ok((config, snapshot))
}
