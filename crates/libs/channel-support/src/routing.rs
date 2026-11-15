//! 通道路由选择契约及模型缓存预览。
use std::{fmt::Write, path::Path};
/// 通道路由选择配置。
///
/// 记录为特定会话选择的服务提供者和模型信息，
/// 以及是否启用了任务模式。用于消息路由决策。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChannelRouteSelection {
    /// 选中的服务提供者名称
    pub provider: String,
    /// 选中的模型标识符
    pub model: String,
    /// 当前作用域选择的推理强度；None 使用 provider 默认值。
    pub reasoning_effort: Option<String>,
    /// 是否启用了任务模式
    pub task_mode_enabled: bool,
}

/// 模型缓存状态。
///
/// 持久化存储已知模型的列表，避免每次启动时重新发现。
#[derive(Debug, Clone, Default, serde::Deserialize)]
struct ModelCacheState {
    /// 缓存条目列表，按提供者分组
    pub entries: Vec<ModelCacheEntry>,
}

/// 模型缓存条目。
///
/// 记录单个提供者下的可用模型列表。
#[derive(Debug, Clone, Default, serde::Deserialize)]
struct ModelCacheEntry {
    /// 提供者名称
    pub provider: String,
    /// 该提供者支持的模型列表
    pub models: Vec<String>,
}


pub const MODEL_CACHE_FILE: &str = "models_cache.json";

pub const MODEL_CACHE_PREVIEW_LIMIT: usize = 10;

pub fn load_cached_model_preview(workspace_dir: &Path, provider_name: &str) -> Vec<String> {
    // 构建缓存文件的完整路径
    let cache_path = workspace_dir.join("state").join(MODEL_CACHE_FILE);
    // 尝试读取缓存文件内容，失败则返回空向量
    let Ok(raw) = std::fs::read_to_string(cache_path) else {
        return Vec::new();
    };
    // 尝试解析 JSON 格式的缓存状态，失败则返回空向量
    let Ok(state) = serde_json::from_str::<ModelCacheState>(&raw) else {
        return Vec::new();
    };

    // 在缓存条目中查找指定的 Provider
    state
        .entries
        .into_iter()
        .find(|entry| entry.provider == provider_name)
        // 提取模型列表，限制预览数量
        .map(|entry| entry.models.into_iter().take(MODEL_CACHE_PREVIEW_LIMIT).collect::<Vec<_>>())
        // 未找到 Provider 时返回空向量
        .unwrap_or_default()
}

pub fn build_models_help_response(
    current: &ChannelRouteSelection,
    workspace_dir: &Path,
) -> String {
    let mut response = String::new();
    // 添加当前路由状态信息
    let _ = writeln!(
        response,
        "Current provider: `{}`\nCurrent model: `{}`",
        current.provider, current.model
    );
    // 添加模型切换命令说明
    response.push_str("\nSwitch model with `/model <model-id>`.\n");
    // 添加授权相关命令说明
    response.push_str("Request supervised tool approval with `/approve-request <tool-name>`.\n");
    response.push_str("Request one-time all-tools approval with `/approve-all-once`.\n");
    response.push_str("Confirm approval with `/approve-confirm <request-id>`.\n");
    response.push_str("Deny approval with `/approve-deny <request-id>`.\n");
    response.push_str("List pending requests with `/approve-pending`.\n");
    response.push_str("Approve supervised tools with `/approve <tool-name>`.\n");
    response.push_str("Revoke approval with `/unapprove <tool-name>`.\n");
    response.push_str("List approval state with `/approvals`.\n");
    // 添加自然语言授权模式的说明
    response.push_str(
        "Natural language also works (policy controlled).\n\
         - `direct` mode (default): `授权工具 shell` grants immediately.\n\
         - `request_confirm` mode: `授权工具 shell` then `确认授权 apr-xxxxxx`.\n",
    );

    // 尝试加载并显示缓存的模型列表
    let cached_models = load_cached_model_preview(workspace_dir, &current.provider);
    if cached_models.is_empty() {
        // 无缓存时，提示用户运行刷新命令
        let _ = writeln!(
            response,
            "\nNo cached model list found for `{}`. Ask the operator to run `shiju models refresh --provider {}`.",
            current.provider, current.provider
        );
    } else {
        // 有缓存时，显示模型预览列表
        let _ = writeln!(response, "\nCached model IDs (top {}):", cached_models.len());
        for model in cached_models {
            let _ = writeln!(response, "- `{model}`");
        }
    }

    response
}

pub fn default_route_selection(defaults: crate::config::ChannelRuntimeDefaults) -> ChannelRouteSelection {
    // 从运行时上下文获取默认配置快照

    // 构建默认路由选择，任务模式默认禁用
    ChannelRouteSelection {
        provider: defaults.default_provider,
        model: defaults.model,
        reasoning_effort: None,
        task_mode_enabled: false,
    }
}

pub fn classify_message_route(
    query_classification: &config_routing::QueryClassificationConfig,
    model_routes: &[config_routing::ModelRouteConfig],
    message: &str,
) -> Option<ChannelRouteSelection> {
    // 使用分类器对消息进行分类，获取决策结果
    // 如果分类功能禁用或消息不匹配任何规则，此处返回 None
    let decision = model_selection::classifier::classify_with_decision(
        query_classification,
        message,
    )?;

    // 在模型路由规则列表中查找与分类提示匹配的路由
    // 使用分类决策的 hint 字段作为匹配键
    let route = model_routes.iter().find(|r| r.hint == decision.hint)?;

    // 记录分类决策日志，便于调试和监控
    tracing::info!(
        target: "query_classification",
        hint = %decision.hint,
        model = %route.model,
        rule_priority = decision.priority,
        message_length = message.len(),
        "Classified message route"
    );

    // 构建并返回路由选择，任务模式禁用
    Some(ChannelRouteSelection {
        provider: route.provider.clone(),
        model: route.model.clone(),
        reasoning_effort: None,
        task_mode_enabled: false,
    })
}

pub fn set_route_selection(
    route_overrides: &std::sync::Mutex<std::collections::HashMap<String, ChannelRouteSelection>>,
    default_route: ChannelRouteSelection,
    sender_key: &str,
    next: ChannelRouteSelection,
) {
    // 获取默认路由作为比较基准

    // 获取 Mutex 锁，处理可能的污染情况
    let mut routes = route_overrides.lock().unwrap_or_else(|e| e.into_inner());
    // 如果新配置等于默认配置，移除自定义条目（恢复默认）
    if next == default_route {
        routes.remove(sender_key);
    } else {
        // 否则，存储新的自定义配置
        routes.insert(sender_key.to_string(), next);
    }
}

use agent_provider::provider;
#[cfg(not(target_arch = "wasm32"))]
pub async fn available_provider_ids() -> Result<Vec<String>, provider::CatalogError> {
    // 异步阻塞调用 provider 模块获取所有已注册的提供者
    let providers = provider::list().await?;
    // 提取所有键（Provider ID）并收集为向量
    let mut ids = providers.into_keys().collect::<Vec<_>>();
    // 按字母顺序排序，确保返回结果的一致性和可预测性
    ids.sort();
    Ok(ids)
}
#[cfg(target_arch = "wasm32")]
pub async fn available_provider_ids() -> Result<Vec<String>, provider::CatalogError> {
    Ok(Vec::new())
}
pub async fn resolve_provider_alias(name: &str) -> Result<Option<String>, provider::CatalogError> {
    // 去除输入名称的首尾空白字符
    let candidate = name.trim();
    // 空字符串直接返回 None，避免无意义的匹配
    if candidate.is_empty() {
        return Ok(None);
    }

    // 遍历所有可用的 Provider ID 进行不区分大小写的匹配
    for id in available_provider_ids().await? {
        if id.eq_ignore_ascii_case(candidate) {
            // 返回规范化的 Provider ID（保持原始注册时的大小写）
            return Ok(Some(id));
        }
    }

    // 未找到匹配项
    Ok(None)
}

use crate::runtime_state::ChannelRuntimeContext;
pub fn get_route_selection(
    ctx: &ChannelRuntimeContext,
    sender_key: &str,
) -> ChannelRouteSelection {
    ctx.route_overrides
        // 获取 Mutex 锁，如果被污染则恢复并继续
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        // 查找指定发送者的自定义路由
        .get(sender_key)
        .cloned()
        // 如果没有自定义配置，使用默认路由
        .unwrap_or_else(|| default_route_selection(crate::config::runtime_defaults_snapshot(ctx)))
}
