//! 配置归一化领域逻辑。
use std::path::PathBuf;
pub use config_aggregate::*;

/// 根据 `default_provider` 指向的命名 provider profile 归一化运行时 provider 字段。
///
/// 该函数会把 profile 中的 `base_url`、`wire_api` 和 OpenAI 兼容认证需求同步到旧的顶层字段，
/// 以便下游仍可通过既有 provider 字段工作。profile 不存在或当前 provider 为空时直接返回。
pub fn apply_named_model_provider_profile(config: &mut Config, read_codex_openai_api_key: fn() -> Option<String>, normalize_wire_api: fn(&str) -> Option<&'static str>) {
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
pub fn apply_workspace_override(config: &mut Config, workspace: &str, resolve_config_dir_for_workspace: fn(&std::path::Path) -> (PathBuf, PathBuf)) {
    if workspace.trim().is_empty() {
        return;
    }

    let (_, workspace_dir) = resolve_config_dir_for_workspace(&PathBuf::from(workspace));
    config.workspace_dir = workspace_dir;
}
