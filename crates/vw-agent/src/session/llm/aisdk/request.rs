//! AI SDK 流式请求处理模块
//!
//! 本模块提供基于 AI SDK 的流式请求处理功能，用于与兼容 OpenAI API 的
//! 语言模型服务商进行通信。主要职责包括：
//!
//! - 认证信息处理（API Key / OAuth Token）
//! - 请求参数构建与配置
//! - 流式响应处理与事件分发
//!
//! # 架构说明
//!
//! 本模块仅在非 WebAssembly 目标平台上可用（`#[cfg(not(target_arch = "wasm32"))]`），
//! 因为 WebAssembly 环境下不支持完整的异步网络栈。
//!
#[cfg(not(target_arch = "wasm32"))]
use serde_json::{Map, Value};
#[cfg(not(target_arch = "wasm32"))]
use sha2::{Digest, Sha256};
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::auth;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::env;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::provider::provider;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::session::llm::logging::LOGGER;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::session::llm::types::{Error, StreamEvent};
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::tools;

#[cfg(not(target_arch = "wasm32"))]
use super::convert::compute_request_info;
#[cfg(not(target_arch = "wasm32"))]
use super::driver::dispatch_stream_request;

#[cfg(not(target_arch = "wasm32"))]
fn redact_bearer_for_log(secret: &str) -> String {
    let chars = secret.chars().collect::<Vec<_>>();
    let len = chars.len();
    if len <= 4 {
        return "*".repeat(len.max(1));
    }
    let prefix = chars.iter().take(2).collect::<String>();
    let suffix = chars.iter().skip(len.saturating_sub(2)).collect::<String>();
    format!("{prefix}***{suffix}")
}

#[cfg(not(target_arch = "wasm32"))]
fn bearer_fingerprint(secret: &str) -> String {
    let digest = Sha256::digest(secret.as_bytes());
    hex::encode(digest)[..12].to_string()
}

/// 执行基于 AI SDK 的流式请求
///
/// 该函数是与语言模型提供商进行流式通信的主要入口点。它处理认证、
/// 请求构建、日志记录，并协调流式响应的事件分发。
///
/// # 参数
///
/// * `provider_info` - 提供商配置信息，包含 ID、密钥配置、环境变量等
/// * `auth_info` - 可选的认证信息，支持 API Key 或 OAuth Token
/// * `headers` - 自定义 HTTP 请求头（注：当前 AI SDK 不支持自定义 headers）
/// * `merged_options` - 合并后的请求选项（JSON 格式）
/// * `model` - 模型配置，包含 API 适配器、URL、模型 ID 等
/// * `chat_messages` - 聊天消息数组（JSON 格式）
/// * `tools` - 工具定义映射表，用于函数调用功能
/// * `temperature` - 可选的采样温度参数（0.0-2.0）
/// * `top_p` - 可选的核采样参数（0.0-1.0）
/// * `max_output_tokens` - 可选的最大输出 token 数量
/// * `retries` - 失败重试次数
/// * `abort` - 可选的中断信号接收器，用于取消请求
/// * `on_event` - 流式事件回调函数，接收每个流事件
///
/// # 返回值
///
/// 返回 `Result<(), Error>`：
/// - `Ok(())` - 流式请求成功完成
/// - `Err(Error)` - 请求失败，包含具体错误类型
///
/// # 认证优先级
///
/// Bearer Token 的获取按以下优先级顺序：
/// 1. 提供商配置中的 `key` 字段
/// 2. 提供商配置的 `env` 环境变量列表中第一个非空值
/// 3. `auth_info` 中的 API Key 或 OAuth Access Token
///
/// # 错误
///
/// 可能返回以下错误：
/// - `Error::Api` - API 相关错误（认证失败、不支持自定义 headers 等）
/// - `Error::Stream` - 流处理错误
///
/// # 示例
///
/// ```ignore
/// use std::collections::HashMap;
/// use serde_json::json;
///
/// let provider = get_provider_info();
/// let model = get_model_config();
/// let messages = json!([{"role": "user", "content": "你好"}]);
/// let tools = HashMap::new();
///
/// let result = do_stream_request_aisdk(
///     &provider,
///     None,
///     &HashMap::new(),
///     &json!({}),
///     &model,
///     &messages,
///     &tools,
///     Some(0.7),
///     None,
///     Some(1024),
///     3,
///     None,
///     &mut |event| {
///         println!("收到事件: {:?}", event);
///     },
/// ).await;
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub async fn do_stream_request_aisdk(
    provider_info: &provider::Info,
    auth_info: Option<&auth::Info>,
    headers: &HashMap<String, String>,
    merged_options: &Value,
    model: &provider::Model,
    chat_messages: &Value,
    tools: &HashMap<String, tools::ToolSpec>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    max_output_tokens: Option<u64>,
    retries: u64,
    abort: Option<&tokio::sync::watch::Receiver<bool>>,
    on_event: &mut (impl FnMut(StreamEvent) + Send),
) -> Result<(), Error> {
    // 检查是否存在自定义 headers（排除 user-agent）
    // AI SDK 不支持自定义 headers，如果有则直接返回错误
    let has_custom_headers = headers.keys().any(|k| !k.eq_ignore_ascii_case("user-agent"));
    if has_custom_headers {
        return Err(Error::Api(crate::app::agent::session::message::AssistantError::Unknown {
            message: "aisdk 不支持自定义 headers".to_string(),
        }));
    }

    // 验证认证信息是否为空，生成对应的错误提示消息
    // 用于在后续无法获取有效 token 时提供更详细的错误原因
    let auth_error_message = match auth_info {
        Some(auth::Info::Api(api)) if api.key.trim().is_empty() => Some("API key 为空"),
        Some(auth::Info::Oauth(o)) if o.access.trim().is_empty() => Some("OAuth access token 为空"),
        _ => None,
    };

    // 按优先级顺序获取 Bearer Token：
    // 1. 提供商配置中的 key 字段
    // 2. 提供商环境变量列表中第一个非空值
    // 3. auth_info 中的认证信息
    let bearer_and_source = provider_info
        .key
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|k| !k.is_empty())
        .map(|key| (key, "provider.key".to_string()))
        .or_else(|| {
            for k in &provider_info.env {
                if let Some(v) = env::get(k).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
                {
                    return Some((v, format!("provider.env:{k}")));
                }
            }
            None
        })
        .or_else(|| match auth_info {
            Some(auth::Info::Api(api)) if !api.key.trim().is_empty() => {
                Some((api.key.trim().to_string(), "auth.api".to_string()))
            }
            Some(auth::Info::Oauth(o)) if !o.access.trim().is_empty() => {
                Some((o.access.trim().to_string(), "auth.oauth".to_string()))
            }
            _ => None,
        });

    // 如果无法获取有效的 Bearer Token，返回认证错误
    let Some((bearer, bearer_source)) = bearer_and_source else {
        return Err(Error::Api(
            crate::app::agent::session::message::AssistantError::ProviderAuthError {
                provider_id: provider_info.id.clone(),
                message: auth_error_message.unwrap_or("API key 为空").to_string(),
            },
        ));
    };

    // 计算请求信息，包括基础 URL、请求 URL、路径覆盖和消息转换
    let request_info = compute_request_info(model, chat_messages)?;
    let provider_id = provider_info.id.clone();

    // 记录请求日志，包含提供商 ID、模型 ID、适配器和 URL 信息
    LOGGER.clone_logger().tag("providerID", &provider_id).tag("modelID", &model.api.id).debug(
        "aisdk.request",
        Some({
            let mut m = Map::new();
            m.insert("adapter".to_string(), Value::String(model.api.adapter.clone()));
            m.insert("apiURL".to_string(), Value::String(model.api.url.clone()));
            m.insert("authSource".to_string(), Value::String(bearer_source.clone()));
            m.insert("authKeyPreview".to_string(), Value::String(redact_bearer_for_log(&bearer)));
            m.insert("authKeyLen".to_string(), Value::from(bearer.chars().count() as u64));
            m.insert("authKeyFingerprint".to_string(), Value::String(bearer_fingerprint(&bearer)));
            // 仅当 base_url 非空时才记录
            if !request_info.base_url.is_empty() {
                m.insert("baseURL".to_string(), Value::String(request_info.base_url.clone()));
            }
            // 仅当存在路径覆盖时才记录
            if let Some(p) = request_info.path_override.as_ref() {
                m.insert("path".to_string(), Value::String(p.to_string()));
            }
            m.insert(
                "messageCount".to_string(),
                Value::from(chat_messages.as_array().map_or(0, |v| v.len()) as u64),
            );
            m.insert("toolCount".to_string(), Value::from(tools.len() as u64));
            if let Some(v) = temperature {
                m.insert("temperature".to_string(), Value::from(v));
            }
            if let Some(v) = top_p {
                m.insert("topP".to_string(), Value::from(v));
            }
            if let Some(v) = max_output_tokens {
                m.insert("maxOutputTokens".to_string(), Value::from(v));
            }
            if let Some(options) = merged_options.as_object() {
                let mut keys = options.keys().cloned().collect::<Vec<_>>();
                keys.sort();
                m.insert(
                    "optionKeys".to_string(),
                    Value::Array(keys.into_iter().map(Value::String).collect()),
                );
            }
            m.insert("requestURL".to_string(), Value::String(request_info.request_url.clone()));
            m
        }),
    );

    dispatch_stream_request(
        &provider_id,
        model,
        &bearer,
        request_info,
        tools,
        temperature,
        top_p,
        max_output_tokens,
        merged_options,
        retries,
        abort,
        on_event,
    )
    .await
}
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "request_tests.rs"]
mod request_tests;
