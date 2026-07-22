//! agent 配置对象的扩展入口。
//!
//! 本模块重新导出共享配置结构，并在 agent 侧为 `Config` 补齐加载、保存、校验和环境变量覆盖能力。
//! 这里还包含少量运行时归一化逻辑，用于把用户选择的模型 provider profile 映射到现有 provider 字段。

use std::path::PathBuf;

use crate::app::agent::config::schema::config_helpers::read_codex_openai_api_key;
use crate::app::agent::config::schema::config_validate::normalize_wire_api;
use crate::app::agent::config::schema::workspace::resolve_config_dir_for_workspace;

pub use vw_config_types::config::*;

use anyhow::Result;

/// `Config` 在 agent 运行时需要的持久化与校验能力。
///
/// trait 将共享配置类型与 agent 侧的 I/O、环境变量覆盖和校验实现连接起来。所有方法都返回
/// `anyhow::Result`，调用方应把错误展示为配置加载或保存失败，而不是继续使用部分初始化的配置。
pub trait ConfigExt {
    /// 从磁盘加载配置，不存在时初始化默认配置。
    ///
    /// 返回完整的 `Config`。加载过程中会处理旧格式迁移、密钥解密、环境变量覆盖和配置校验；任一阶段失败
    /// 都会返回错误。
    fn load_or_init() -> impl std::future::Future<Output = Result<Config>> + Send;

    /// 将当前配置保存到本地租户的共享数据库。
    ///
    /// 保存前由底层实现处理校验、密钥加密和 JSON payload 写入；数据库或序列化失败会作为错误返回。
    fn save(&self) -> impl std::future::Future<Output = Result<()>> + Send;

    /// 校验当前配置是否满足运行时约束。
    ///
    /// 返回值为 `Ok(())` 表示配置可用；字段为空、取值越界或安全策略无效时返回错误。
    fn validate(&self) -> Result<()>;

    /// 应用当前进程环境变量中的配置覆盖。
    ///
    /// 该方法只修改内存中的配置对象，不负责保存到磁盘。
    fn apply_env_overrides(&mut self);
}

impl ConfigExt for Config {
    fn load_or_init() -> impl std::future::Future<Output = Result<Config>> + Send {
        crate::app::agent::config::schema::config_load::load_or_init_config()
    }

    fn save(&self) -> impl std::future::Future<Output = Result<()>> + Send {
        crate::app::agent::config::schema::config_save::save_config(self)
    }

    fn validate(&self) -> Result<()> {
        crate::app::agent::config::schema::config_validate::validate_config(self)
    }

    fn apply_env_overrides(&mut self) {
        crate::app::agent::config::schema::config_env::apply_env_overrides(self);
    }
}

/// 根据 `default_provider` 指向的命名 provider profile 归一化运行时 provider 字段。
///
/// 该函数会把 profile 中的 `base_url`、`wire_api` 和 OpenAI 兼容认证需求同步到旧的顶层字段，
/// 以便下游仍可通过既有 provider 字段工作。profile 不存在或当前 provider 为空时直接返回。
pub(crate) fn apply_named_model_provider_profile(config: &mut super::Config) {
    let Some(current_provider) = config.default_provider.clone() else {
        return;
    };

    let Some((profile_key, profile)) = config.lookup_model_provider_profile(&current_provider)
    else {
        return;
    };

    let base_url = profile
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    if config.api_url.as_deref().map(str::trim).is_none_or(|value| value.is_empty()) {
        if let Some(base_url) = base_url.as_ref() {
            config.api_url = Some(base_url.clone());
        }
    }

    if profile.requires_openai_auth
        && config.api_key.as_deref().map(str::trim).is_none_or(|value| value.is_empty())
    {
        // 需要 OpenAI 认证的 profile 尽量复用 Codex 既有凭据，但只在当前配置没有显式密钥时填充。
        let codex_key = std::env::var("OPENAI_API_KEY")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .or_else(read_codex_openai_api_key);
        if let Some(codex_key) = codex_key {
            config.api_key = Some(codex_key);
        }
    }

    let normalized_wire_api = profile.wire_api.as_deref().and_then(normalize_wire_api);
    let profile_name = profile.name.as_deref().map(str::trim).filter(|value| !value.is_empty());

    if normalized_wire_api == Some("responses") {
        // Responses API 走专用 provider，避免自定义 base_url 路径误用 Chat Completions 协议。
        config.default_provider = Some("openai-codex".to_string());
        return;
    }

    if let Some(profile_name) = profile_name {
        if !profile_name.eq_ignore_ascii_case(&profile_key) {
            config.default_provider = Some(profile_name.to_string());
            return;
        }
    }

    if let Some(base_url) = base_url {
        config.default_provider = Some(format!("custom:{base_url}"));
    }
}

/// 将命令行或调用方指定的 workspace 覆盖写入配置。
///
/// 空白 workspace 会被忽略；非空路径会按 workspace 解析规则转换为运行时使用的 workspace 目录。
pub(crate) fn apply_workspace_override(config: &mut super::Config, workspace: &str) {
    if workspace.trim().is_empty() {
        return;
    }

    let (_, workspace_dir) = resolve_config_dir_for_workspace(&PathBuf::from(workspace));
    config.workspace_dir = workspace_dir;
}
