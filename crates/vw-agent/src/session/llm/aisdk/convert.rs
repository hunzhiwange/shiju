//! AI SDK 请求转换模块
//!
//! 本模块提供将内部模型配置和消息格式转换为 AI SDK 兼容格式的功能。
//! 主要职责包括：
//! - 计算和规范化 API 请求的基础 URL 和请求路径
//! - 检测模型适配器类型并应用相应的端点规则
//! - 将请求转换为 AI SDK 所需的结构
//!
//! # 条件编译
//!
//! 本模块仅在非 WASM 目标平台上可用，因为相关功能依赖标准库的集合类型
//! 和 JSON 处理，这些在 WASM 环境下可能有不同的实现需求。

#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::provider::provider;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::session::llm::types::Error;
#[cfg(not(target_arch = "wasm32"))]
use serde_json::Value;

#[cfg(not(target_arch = "wasm32"))]
use super::util::openai_messages_to_aisdk_messages;

/// AI SDK 请求信息容器
///
/// 包含构建和发送 AI SDK 请求所需的所有元数据和转换后的消息。
/// 该结构体由 [`compute_request_info`] 函数生成，封装了请求的 URL 配置和消息转换结果。
///
/// # 字段说明
///
/// - `base_url`: API 服务器的基础 URL，不包含具体的端点路径
/// - `request_url`: 完整的请求 URL，包含基础 URL 和端点路径
/// - `path_override`: 可选的路径覆盖值，当原始 URL 包含特定端点时使用
/// - `enforce_strict_tool_schema`: 是否强制执行严格的工具 schema 规范化，
///   对于 GPT 类模型使用 OpenAI 适配器时为 true
/// - `messages`: 转换为 AI SDK 消息格式的聊天消息列表
#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct AisdkRequestInfo {
    /// API 服务器基础 URL（不含端点路径）
    pub(crate) base_url: String,
    /// 完整的请求 URL（基础 URL + 端点路径）
    pub(crate) request_url: String,
    /// 可选的端点路径覆盖值
    pub(crate) path_override: Option<String>,
    /// 是否强制执行严格的工具 schema 规范化
    pub(crate) enforce_strict_tool_schema: bool,
    /// 转换后的 AI SDK 格式消息列表
    pub(crate) messages: Vec<aisdk::core::Message>,
}

/// 计算并构建 AI SDK 请求信息
///
/// 根据模型配置和聊天消息，生成包含完整请求元数据的 `AisdkRequestInfo`。
/// 该函数处理多种适配器类型和端点格式的规范化，确保请求能够正确路由到目标 API。
///
/// # 参数
///
/// - `model`: 模型配置引用，包含 API URL、适配器类型、模型 ID 和名称等信息
/// - `chat_messages`: OpenAI 格式的聊天消息 JSON 值
///
/// # 返回值
///
/// - `Ok(AisdkRequestInfo)`: 成功时返回包含请求元数据的结构体
/// - `Err(Error)`: 消息转换失败时返回错误
///
/// # 处理逻辑
///
/// 1. **适配器检测**: 识别 OpenAI 和 OpenAI 兼容适配器
/// 2. **GPT 模型检测**: 判断是否为 GPT 系列模型以决定是否强制工具 schema
/// 3. **端点解析**: 从 URL 中提取端点类型（chat/completions、responses、chat/stream）
/// 4. **URL 规范化**:
///    - 移除 URL 中的片段和查询参数
///    - 对于 OpenAI 适配器，确保基础 URL 包含 `/v1` 版本后缀
///    - 处理已有端点路径的 URL，正确拆分基础 URL 和路径
/// 5. **消息转换**: 将 OpenAI 消息格式转换为 AI SDK 格式
///
/// # 示例
///
/// ```ignore
/// let model = provider::Model { /* ... */ };
/// let messages = serde_json::json!([{"role": "user", "content": "Hello"}]);
/// let info = compute_request_info(&model, &messages)?;
/// println!("Request URL: {}", info.request_url);
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn compute_request_info(
    model: &provider::Model,
    chat_messages: &Value,
) -> Result<AisdkRequestInfo, Error> {
    if model.api.adapter.trim() == "anthropic"
        || model.api.url.trim().trim_end_matches('/').ends_with("/messages")
    {
        let base_url =
            model.api.url.trim().trim_end_matches('/').trim_end_matches("/messages").to_string();
        return Ok(AisdkRequestInfo {
            request_url: format!("{base_url}/messages"),
            base_url,
            path_override: Some("messages".into()),
            enforce_strict_tool_schema: false,
            messages: openai_messages_to_aisdk_messages(chat_messages, model)?,
        });
    }
    // 提取并规范化 API URL，移除首尾空白和末尾斜杠
    let api_url = model.api.url.trim().trim_end_matches('/').to_string();
    // 获取适配器类型并规范化为小写
    let adapter = model.api.adapter.trim();
    // 将模型 ID 和名称转换为小写用于模式匹配
    let model_id_lower = model.id.to_ascii_lowercase();
    let model_name_lower = model.name.to_ascii_lowercase();
    // 检测是否为 GPT 系列模型（通过 ID 或名称判断）
    let is_gpt_like = model_id_lower.contains("gpt") || model_name_lower.contains("gpt");
    let is_openai_compatible_adapter = matches!(
        adapter,
        "openai"
            | "openai-compatible"
            | "openai-responses"
            | "acp"
            | "agent-client-protocol"
            | "agent_client_protocol"
    );
    // 对于 OpenAI 兼容适配器的 GPT 模型，强制执行严格的工具 schema
    let enforce_strict_tool_schema = is_openai_compatible_adapter && is_gpt_like;

    // 端点探测辅助函数：移除 URL 中的片段（#）和查询参数（?），并清理末尾斜杠
    // 用于从可能包含额外参数的 URL 中提取纯净的端点路径
    let endpoint_probe = |u: &str| {
        u.split('#')
            .next()
            .unwrap_or(u)
            .split('?')
            .next()
            .unwrap_or(u)
            .trim_end_matches('/')
            .to_string()
    };

    // 端点类型检测函数：根据 URL 路径后缀识别端点类型
    // 支持：/chat/completions、/responses、/chat/stream 三种端点
    let endpoint_kind = |u: &str| {
        let p = endpoint_probe(u);
        if p.ends_with("/chat/completions") {
            Some("chat_completions")
        } else if p.ends_with("/responses") {
            Some("responses")
        } else if p.ends_with("/chat/stream") {
            Some("chat_stream")
        } else {
            None
        }
    };

    // 从 URL 中拆分出基础 URL（移除已知的端点路径后缀）
    // 这样可以从包含完整端点的 URL 中提取出服务器基础地址
    let split_endpoint_base = |u: &str| {
        let p = endpoint_probe(u);
        if let Some(v) = p.strip_suffix("/chat/completions") {
            v.to_string()
        } else if let Some(v) = p.strip_suffix("/responses") {
            v.to_string()
        } else if let Some(v) = p.strip_suffix("/chat/stream") {
            v.to_string()
        } else {
            p
        }
    };

    // 计算基础 URL：
    // 对于 OpenAI 和兼容适配器，需要规范化 URL 结构
    // - 先移除已有的端点路径
    // - 检查是否已包含版本后缀（如 /v1）
    // - 如果没有版本后缀，添加 /v1
    let base_url = if is_openai_compatible_adapter {
        let base = split_endpoint_base(&api_url);
        // 获取 URL 路径的最后一个段用于版本检测
        let last = base.rsplit('/').next().unwrap_or_default();
        // 检测版本后缀格式：'v' 后跟纯数字（如 v1、v2）
        let has_version_suffix = last.len() > 1
            && last.starts_with('v')
            && last[1..].chars().all(|c| c.is_ascii_digit());
        // 如果已有版本后缀则保持原样，否则添加 /v1
        if has_version_suffix { base } else { format!("{}/v1", base) }
    } else {
        // 非OpenAI适配器直接使用原始URL
        api_url.clone()
    };

    // 确定路径覆盖值：
    // 根据检测到的端点类型或适配器配置，决定是否需要覆盖默认的请求路径
    let path_override = if matches!(endpoint_kind(&api_url), Some("chat_stream")) {
        // chat/stream 端点映射到标准的 chat/completions
        Some("chat/completions".to_string())
    } else if matches!(endpoint_kind(&api_url), Some("chat_completions")) {
        // 已明确指定 chat/completions 端点
        Some("chat/completions".to_string())
    } else if adapter == "openai-responses" || matches!(endpoint_kind(&api_url), Some("responses"))
    {
        // responses 端点（用于某些特定 API）
        Some("responses".to_string())
    } else if is_openai_compatible_adapter && is_gpt_like {
        // GPT 模型使用 OpenAI 兼容适配器时，默认使用 chat/completions
        Some("chat/completions".to_string())
    } else {
        // 其他情况不覆盖路径
        None
    };

    // 构建完整的请求 URL：
    // 如果有路径覆盖则拼接基础 URL 和覆盖路径，否则使用默认的 chat/completions
    let request_url = if let Some(p) = path_override.as_ref() {
        format!("{}/{}", base_url.trim_end_matches('/'), p.trim_start_matches('/'))
    } else {
        format!("{}/chat/completions", base_url.trim_end_matches('/'))
    };

    // 确保基础 URL 非空（防御性编程）
    let base_url = if base_url.is_empty() { String::new() } else { base_url };

    // 对于非 OpenAI 适配器，需要重新评估基础 URL
    // 检查原始 API URL 的最后一个路径段是否为空，以决定是否使用处理后的 base_url
    let base_url = if is_openai_compatible_adapter {
        base_url
    } else {
        let last = api_url.rsplit('/').next().unwrap_or_default();
        if last.is_empty() { api_url.clone() } else { base_url }
    };

    // 将 OpenAI 格式的消息转换为 AI SDK 格式
    let messages = openai_messages_to_aisdk_messages(chat_messages, model)?;

    // 返回构建完成的请求信息
    Ok(AisdkRequestInfo {
        base_url,
        request_url,
        path_override,
        enforce_strict_tool_schema,
        messages,
    })
}
#[cfg(test)]
#[path = "convert_tests.rs"]
mod convert_tests;
