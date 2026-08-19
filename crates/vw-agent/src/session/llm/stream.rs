//! LLM 流式响应处理模块
//!
//! 本模块提供与大语言模型（LLM）进行流式交互的核心功能。主要职责包括：
//! - 构建和发送流式请求到各种 AI 提供商
//! - 处理流式响应事件（如 token 生成、错误、中断等）
//! - 管理请求参数合并（provider、model、agent、user 四层配置）
//! - 实现重试机制和错误处理
//! - 支持 OpenAI/OpenAI-compatible 适配器
//!
//! # 主要组件
//!
//! - [`stream`]: 核心流式请求函数，处理完整的请求生命周期
//! - [`should_abort`]: 检查请求是否应被中断
//! - [`is_retryable_assistant_error`]: 判断错误是否可重试
//!
//! # 架构说明
//!
//! 流式请求采用事件驱动模式：
//! 1. 接收 `StreamInput` 输入参数
//! 2. 并行获取配置、provider 信息、认证信息
//! 3. 合并多层配置选项
//! 4. 构建请求消息和工具定义
//! 5. 通过回调函数实时推送 `StreamEvent` 事件
//!
//! # 使用示例
//!
//! ```ignore
//! use crate::app::agent::session::llm::stream::{stream, StreamInput, StreamEvent};
//!
//! let input = StreamInput { /* ... */ };
//! stream(input, |event| {
//!     match event {
//!         StreamEvent::Token(token) => print!("{}", token),
//!         StreamEvent::Error(err) => eprintln!("错误: {:?}", err),
//!         _ => {}
//!     }
//! }).await?;
//! ```

use crate::app::agent::auth;
use crate::app::agent::config;
use crate::app::agent::installation;
use crate::app::agent::project::instance;
use crate::app::agent::provider::provider;
use crate::app::agent::provider::transform as provider_transform;
use crate::app::agent::session::message;
use crate::app::agent::tools;
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use super::acp::request::do_stream_request_acp;
#[cfg(not(target_arch = "wasm32"))]
use super::aisdk::do_stream_request_aisdk;
use super::logging::LOGGER;
use super::messages::{build_chat_messages, build_system_messages, has_tool_calls, resolve_tools};
use super::options::merge_deep_value;
use super::types::{Error, StreamEvent, StreamInput};

fn request_options_map_enable_acp(options: &HashMap<String, Value>) -> bool {
    options.get("acp_test").and_then(Value::as_bool).unwrap_or(false)
        || options
            .get("acp_agent")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
}

fn request_options_value_enable_acp(options: &Value) -> bool {
    options.get("acp_test").and_then(Value::as_bool).unwrap_or(false)
        || options
            .get("acp_agent")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
}

fn acp_route_provider_info() -> provider::Info {
    provider::Info {
        id: "acp".to_string(),
        name: "ACP".to_string(),
        source: provider::ProviderSource::Config,
        env: Vec::new(),
        key: None,
        options: HashMap::new(),
        models: HashMap::new(),
    }
}

fn resolve_stream_provider_info(
    provider_id: &str,
    provider_info: Option<provider::Info>,
    acp_requested: bool,
) -> Result<provider::Info, Error> {
    match provider_info {
        Some(provider_info) => Ok(provider_info),
        None if acp_requested && provider_id == "acp" => Ok(acp_route_provider_info()),
        None => Err(Error::ProviderNotFound(provider_id.to_string())),
    }
}

/// 执行 LLM 流式请求的核心函数
///
/// 该函数是会话层与 AI 提供商交互的主要入口点，负责：
/// 1. 初始化日志记录器并标记关键上下文信息
/// 2. 并行获取配置、provider 信息和认证信息
/// 3. 构建系统消息和用户消息
/// 4. 合并多层配置选项（provider → model → agent → user variant）
/// 5. 解析并准备工具定义
/// 6. 设置请求头（包括认证和自定义头）
/// 7. 执行流式请求并处理重试逻辑
///
/// # 参数
///
/// - `input`: 流式请求的输入参数，包含：
///   - `model`: 模型配置（provider ID、model ID、capabilities 等）
///   - `agent`: Agent 配置（name、mode、temperature、top_p、options 等）
///   - `user`: 用户信息（ID、variant、权限等）
///   - `messages`: 对话历史消息
///   - `tools`: 可用工具定义
///   - `session_id`: 会话标识符
///   - `abort`: 可选的中断信号接收器
///   - `retries`: 默认重试次数
///   - `small`: 是否使用小型模型标志
///
/// - `on_event`: 事件回调函数，用于处理流式响应事件
///   - 接收 [`StreamEvent`] 枚举值
///   - 必须是 `Send + 'static` 以支持异步跨线程传递
///   - 可能接收到的事件类型：
///     - `StreamEvent::Token`: 生成的文本 token
///     - `StreamEvent::FullMessages`: 完整的消息列表
///     - `StreamEvent::Error`: 错误信息
///     - `StreamEvent::ToolCall`: 工具调用请求
///     - `StreamEvent::Finish`: 流式响应结束
///
/// # 返回值
///
/// - `Ok(())`: 流式请求成功完成
/// - `Err(Error::ProviderNotFound)`: 指定的 provider 不存在
/// - `Err(Error::Api(err))`: API 调用失败（经过重试后仍失败）
/// - `Err(Error::Aborted)`: 请求被用户中断
/// - `Err(Error::UnknownAdapter)`: 不支持的 adapter 类型
///
/// # 配置合并策略
///
/// 配置选项按以下优先级从低到高合并（后者覆盖前者）：
/// 1. Provider 默认配置（`provider_info.options`）
/// 2. Model 特定配置（`input.model.options`）
/// 3. Agent 配置（`input.agent.options`）
/// 4. User variant 配置（`input.model.variants[variant]`，仅在非 small 模式下）
///
/// # 重试机制
///
/// - 遇到可重试错误时自动重试
/// - 重试次数由 `merged_options.max_retries` 或 `input.retries` 决定
/// - 重试延迟使用指数退避策略（由 [`crate::app::agent::session::retry::delay`] 计算）
/// - 只有 [`is_retryable_assistant_error`] 返回 true 的错误才会重试
///
/// # 特殊处理
///
/// - **Codex 模式**: 当 provider 为 OpenAI 且使用 OAuth 认证时，会移除 `instructions` 选项
/// - **LiteLLM 代理**: 当检测到 LiteLLM 代理且消息历史包含工具调用但当前无活跃工具时，
///   会添加一个空的 `_noop` 工具以确保兼容性
/// - **GitHub Copilot**: 对包含 `github-copilot` 的 provider 不设置 max_output_tokens
/// - **VibeWindow provider**: 添加自定义请求头（`x-vibewindow-project` 等）
/// - **WASM 平台**: 不支持流式请求，直接返回错误
///
/// # 错误处理
///
/// 函数会在以下情况下返回错误：
/// - Provider 不存在
/// - Adapter 不是 `openai` 或 `openai-compatible`（特定 provider 除外）
/// - 在 WASM 平台上运行
/// - 请求被中断（通过 `abort` 信号）
/// - API 调用失败且重试次数耗尽
///
/// # 示例
///
/// ```ignore
/// let input = StreamInput {
///     model: ModelConfig { provider_id: "openai".into(), id: "gpt-4".into(), ... },
///     agent: AgentInfo { name: "assistant".into(), mode: "chat".into(), ... },
///     messages: vec![Message { role: "user".into(), content: "你好".into(), ... }],
///     session_id: "session-123".into(),
///     user: UserInfo { id: "user-456".into(), ... },
///     tools: HashMap::new(),
///     abort: None,
///     retries: 3,
///     small: false,
/// };
///
/// let result = stream(input, |event| {
///     match event {
///         StreamEvent::Token(token) => print!("{}", token),
///         StreamEvent::Finish => println!("\n完成"),
///         StreamEvent::Error(e) => eprintln!("错误: {:?}", e),
///         _ => {}
///     }
/// }).await;
///
/// match result {
///     Ok(()) => println!("请求成功"),
///     Err(e) => eprintln!("请求失败: {:?}", e),
/// }
/// ```
///
/// # 并发安全
///
/// 该函数是异步的且线程安全的，可以在多个并发任务中同时调用。
/// 每次调用都会创建独立的日志上下文和配置合并过程。
pub async fn stream(
    input: StreamInput,
    mut on_event: impl FnMut(StreamEvent) + Send + 'static,
) -> Result<(), Error> {
    // 初始化日志记录器，添加关键上下文标签以便追踪和调试
    let l = LOGGER
        .clone_logger()
        .tag("providerID", &input.model.provider_id)
        .tag("modelID", &input.model.id)
        .tag("sessionID", &input.session_id)
        .tag("small", &input.small.to_string())
        .tag("agent", &input.agent.name)
        .tag("mode", &input.agent.mode);

    // 记录流式请求开始，包含模型和 provider 信息
    l.info(
        "stream",
        Some({
            let mut m = Map::new();
            m.insert("modelID".to_string(), Value::String(input.model.id.clone()));
            m.insert("providerID".to_string(), Value::String(input.model.provider_id.clone()));
            m
        }),
    );

    // 并行获取三个关键配置信息，提高初始化效率
    // - cfg: 全局配置
    // - provider_info: 提供商特定配置和端点信息
    // - auth_info: 认证信息（API key、OAuth token 等）
    let acp_requested_by_input = request_options_map_enable_acp(&input.agent.options);
    let (cfg, provider_info, auth_info) = tokio::join!(
        async { config::get().await },
        async { provider::get_provider(&input.model.provider_id).await },
        async { auth::get(&input.model.provider_id) },
    );
    #[cfg(not(target_arch = "wasm32"))]
    let multimodal_config = cfg.multimodal.clone();
    #[cfg(target_arch = "wasm32")]
    let _ = cfg;

    // 确保提供商存在，否则返回错误
    let provider_info = resolve_stream_provider_info(
        &input.model.provider_id,
        provider_info,
        acp_requested_by_input,
    )?;

    // 检测是否为 Codex 模式（OpenAI provider + OAuth 认证）
    // Codex 模式有特殊的参数处理逻辑
    let is_codex =
        input.model.provider_id == "openai" && matches!(auth_info, Some(auth::Info::Oauth(_)));

    // 构建系统消息，包括 agent 指令、上下文和工具定义
    let system_msgs = build_system_messages(&input, is_codex);

    // ========== 配置选项合并 ==========
    // 按照优先级从低到高合并配置：
    // 1. Provider 默认配置（基础层）
    // 2. Model 特定配置（覆盖 provider 层）
    // 3. Agent 配置（覆盖 model 层）
    // 4. User variant 配置（最高优先级，仅在非 small 模式下）

    let mut merged_options = Value::Object(Map::new());

    // 第一层：合并 Provider 默认配置
    merge_deep_value(
        &mut merged_options,
        &Value::Object(provider_info.options.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
    );

    // 第二层：合并 Model 特定配置
    merge_deep_value(
        &mut merged_options,
        &Value::Object(input.model.options.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
    );

    // 第三层：合并 Agent 配置
    merge_deep_value(
        &mut merged_options,
        &Value::Object(input.agent.options.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
    );

    // 第四层：如果用户指定了 variant 且非 small 模式，合并 variant 特定配置
    // 这允许为不同用户群体或场景提供定制化选项
    if let Some(variant) = input.user.variant.as_ref() {
        if !input.small {
            if let Some(opts) = input.model.variants.get(variant) {
                merge_deep_value(
                    &mut merged_options,
                    &Value::Object(opts.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
                );
            }
        }
    }

    // Codex 模式特殊处理：移除 instructions 选项
    // Codex API 不支持该参数，必须从合并后的选项中删除
    if is_codex {
        if let Some(obj) = merged_options.as_object_mut() {
            obj.remove("instructions");
        }
    }

    // ========== 提取 LLM 参数 ==========

    // Temperature 参数：控制生成文本的随机性
    // - 仅当模型支持 temperature 能力时才设置
    // - 优先使用 agent.temperature，回退到合并配置中的 temperature
    let temperature = if input.model.capabilities.temperature {
        input
            .agent
            .temperature
            .or_else(|| merged_options.get("temperature").and_then(Value::as_f64))
    } else {
        None
    };

    // Top-p 参数：控制核采样（nucleus sampling）
    // - 优先使用 agent.top_p，回退到合并配置中的 top_p
    let top_p = input.agent.top_p.or_else(|| merged_options.get("top_p").and_then(Value::as_f64));

    // Max output tokens 参数：控制生成的最大 token 数
    // - Codex 和 GitHub Copilot 不支持此参数
    // - 支持多个配置键名：max_output_tokens、max_completion_tokens、max_tokens
    // - 逻辑：取默认限制和覆盖限制的最小值
    let max_output_tokens = if is_codex || input.model.provider_id.contains("github-copilot") {
        None
    } else {
        // 获取模型的默认输出限制（来自 provider 配置）
        let default_limit = provider_transform::max_output_tokens(input.model.limit.output);

        // 从合并配置中获取覆盖限制，按优先级尝试不同的键名
        let override_limit = merged_options
            .get("max_output_tokens")
            .and_then(Value::as_u64)
            .or_else(|| merged_options.get("max_completion_tokens").and_then(Value::as_u64))
            .or_else(|| merged_options.get("max_tokens").and_then(Value::as_u64));

        // 根据 default_limit 和 override_limit 决定最终值
        match (default_limit, override_limit) {
            (_, Some(0)) => None,           // 覆盖值为 0 表示无限制
            (0, Some(v)) => Some(v),        // 默认值为 0（无限制），使用覆盖值
            (0, None) => None,              // 两者都为 0（无限制）
            (d, Some(v)) => Some(v.min(d)), // 取最小值确保不超过模型限制
            (d, None) => Some(d),           // 仅使用默认值
        }
    };

    // 重试次数：优先使用配置中的 max_retries，回退到输入参数中的 retries
    let retries =
        merged_options.get("max_retries").and_then(Value::as_u64).unwrap_or(input.retries);

    // ========== 工具准备 ==========

    // 解析并过滤工具定义：
    // - 根据权限和用户设置过滤工具
    // - 如果模型不支持工具调用（toolcall capability），清空工具列表
    let mut tools = resolve_tools(&input.tools, &input.agent.permission, &input.user);
    if !input.model.capabilities.toolcall {
        tools.clear();
    }

    // 检测是否为 LiteLLM 代理环境
    // LiteLLM 是一个统一的 LLM 代理，可能需要特殊处理
    let is_litellm_proxy = provider_info.options.get("litellmProxy").and_then(Value::as_bool)
        == Some(true)
        || input.model.provider_id.to_ascii_lowercase().contains("litellm")
        || input.model.api.id.to_ascii_lowercase().contains("litellm");

    // LiteLLM/Anthropic 代理兼容性修复：
    // 如果消息历史包含工具调用但当前无活跃工具，添加一个空的 _noop 工具
    // 这是为了满足某些代理服务器的要求（必须提供至少一个工具定义）
    if is_litellm_proxy && tools.is_empty() && has_tool_calls(&input.messages) {
        tools.insert(
            "_noop".to_string(),
            tools::ToolSpec::new(
                "_noop",
                "Placeholder for LiteLLM/Anthropic proxy compatibility - required when message history contains tool calls but no active tools are needed",
                json!({ "type": "object", "properties": {} }),
            ),
        );
    }

    // ========== 请求头设置 ==========

    let mut headers: HashMap<String, String> = HashMap::new();

    // 根据 provider 类型设置不同的请求头
    if input.model.provider_id.starts_with("vibewindow") {
        // VibeWindow 原生 provider：添加项目、会话、请求追踪头
        // 这些头用于服务端日志追踪和请求关联
        if let Some(project_id) = instance::project().map(|p| p.id) {
            headers.insert("x-vibewindow-project".to_string(), project_id);
        }
        headers.insert("x-vibewindow-session".to_string(), input.session_id.clone());
        headers.insert("x-vibewindow-request".to_string(), input.user.id.clone());
        headers.insert(
            "x-vibewindow-client".to_string(),
            crate::app::agent::flag::vibewindow_client(),
        );
    } else if input.model.provider_id != "anthropic" {
        // 其他 provider（除 Anthropic 外）：设置标准 User-Agent
        // Anthropic 有自己的认证机制，不需要 User-Agent
        headers.insert("User-Agent".to_string(), installation::user_agent());
    }

    // 合并模型配置中的自定义请求头（可覆盖默认头）
    for (k, v) in &input.model.headers {
        headers.insert(k.clone(), v.clone());
    }

    // ========== Adapter 兼容性检查 ==========

    // 验证 adapter 类型：aisdk 流式请求仅支持 OpenAI 和 OpenAI-compatible adapter
    // 特例：zai-coding-plan 和 zhipuai-coding-plan 提供商不受此限制
    let adapter = input.model.api.adapter.trim();
    let force_acp = request_options_value_enable_acp(&merged_options);
    let is_acp_adapter =
        force_acp || matches!(adapter, "acp" | "agent-client-protocol" | "agent_client_protocol");
    let request_retries = if is_acp_adapter { 0 } else { retries };
    if is_acp_adapter {
        tracing::info!(
            target: "vw_agent",
            model = %input.model.api.id,
            provider = %input.model.provider_id,
            adapter,
            force_acp,
            request_retries,
            requested_acp_agent = merged_options
                .get("acp_agent")
                .and_then(|value| value.as_str())
                .unwrap_or_default(),
            "routing chat stream through ACP"
        );
    }
    let is_openai_compatible_adapter = matches!(
        adapter,
        "openai" | "openai-compatible" | "acp" | "agent-client-protocol" | "agent_client_protocol"
    ) || is_acp_adapter;
    if !is_openai_compatible_adapter
        && input.model.provider_id != "zai-coding-plan"
        && input.model.provider_id != "zhipuai-coding-plan"
    {
        let err = message::AssistantError::Unknown {
            message: format!(
                "aisdk streaming 仅支持 openai/openai-compatible/acp adapter：{adapter}"
            ),
        };
        on_event(StreamEvent::Error(err.clone()));
        return Err(Error::Api(err));
    }

    // ========== 平台兼容性检查 ==========

    // WASM 平台不支持流式请求，直接返回错误
    // 这是由于浏览器环境的限制和 aisdk 的实现方式
    #[cfg(target_arch = "wasm32")]
    {
        let err = message::AssistantError::Unknown { message: "Not supported on Web".to_string() };
        on_event(StreamEvent::Error(err.clone()));
        return Err(Error::Api(err));
    }

    // ========== 执行流式请求（非 WASM 平台）==========

    #[cfg(not(target_arch = "wasm32"))]
    {
        // 构建完整的聊天消息列表（系统消息 + 用户/助手消息）
        let chat_messages = build_chat_messages(&system_msgs, &input.messages, &input.model);
        // 将用户消息中的 `[IMAGE:...]` 本地/远程引用归一化为 data URI，供 aisdk 多模态转换使用。
        let chat_messages =
            match crate::app::agent::multimodal::prepare_openai_json_messages_for_provider(
                chat_messages,
                &multimodal_config,
            )
            .await
            {
                Ok(messages) => {
                    if let Err(message) = assert_no_raw_image_paths(&messages) {
                        let err = message::AssistantError::Unknown { message };
                        on_event(StreamEvent::Error(err.clone()));
                        return Err(Error::Api(err));
                    }
                    messages
                }
                Err(error) => {
                    let err = message::AssistantError::Unknown {
                        message: format!("multimodal image preparation failed: {error}"),
                    };
                    on_event(StreamEvent::Error(err.clone()));
                    return Err(Error::Api(err));
                }
            };

        // 发送完整消息事件，允许调用方查看实际发送给 API 的消息
        if let Value::Array(msgs) = chat_messages.clone() {
            on_event(StreamEvent::FullMessages(msgs));
        }

        // 准备 aisdk 选项：从合并配置中移除 litellmProxy（这是元数据，不应传给 API）
        let mut aisdk_options = merged_options.clone();
        if let Some(obj) = aisdk_options.as_object_mut() {
            obj.remove("litellmProxy");
            // This flag controls the local gateway preprocessing boundary and
            // must not be forwarded as an unknown provider request option.
            obj.remove(crate::app::agent::multimodal::DISABLE_OCR_OPTION);
        }

        // ========== 重试循环 ==========
        let mut attempt = 0u64;
        loop {
            // 在每次尝试前检查是否应该中断请求
            if should_abort(input.abort.as_ref()) {
                on_event(StreamEvent::Error(message::AssistantError::MessageAbortedError {
                    message: "aborted".to_string(),
                }));
                return Err(Error::Aborted);
            }

            attempt += 1;

            // 执行流式请求
            let res = if is_acp_adapter {
                do_stream_request_acp(
                    &input.model,
                    &provider_info,
                    auth_info.as_ref(),
                    &aisdk_options,
                    &chat_messages,
                    &input.session_id,
                    input.abort.as_ref(),
                    &mut on_event,
                )
                .await
            } else {
                do_stream_request_aisdk(
                    &provider_info,
                    auth_info.as_ref(),
                    &headers,
                    &aisdk_options,
                    &input.model,
                    &chat_messages,
                    &tools,
                    temperature,
                    top_p,
                    max_output_tokens,
                    retries,
                    input.abort.as_ref(),
                    &mut on_event,
                )
                .await
            };

            // 处理请求结果
            match res {
                Ok(()) => return Ok(()), // 请求成功，返回
                Err(Error::Api(err)) => {
                    // API 错误：检查是否可重试
                    let retry_allowed = if is_acp_adapter {
                        is_acp_retryable_assistant_error(&err)
                    } else {
                        is_retryable_assistant_error(&err)
                    };
                    let retry_budget_exhausted = if is_acp_adapter {
                        attempt > request_retries
                    } else {
                        attempt > request_retries.saturating_add(1)
                    };
                    if !retry_allowed || retry_budget_exhausted {
                        // 不可重试或重试次数耗尽，返回错误
                        on_event(StreamEvent::Error(err.clone()));
                        return Err(Error::Api(err));
                    }
                    // 计算重试延迟（指数退避）
                    let delay_ms = crate::app::agent::session::retry::delay(attempt, Some(&err));
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
                Err(e) => return Err(e), // 其他错误直接返回
            }
        }
    }
}

/// 检查请求是否应该被中断
///
/// 该函数用于在流式请求过程中检查用户是否请求中断。
/// 通过检查 watch channel 的当前值来判断中断状态。
///
/// # 参数
///
/// - `rx`: 可选的 watch channel 接收器引用
///   - `Some(receiver)`: 监听中断信号的接收器
///   - `None`: 无中断机制，始终返回 false
///
/// # 返回值
///
/// - `true`: 请求应该被中断（接收器存在且值为 true）
/// - `false`: 请求应继续执行（无接收器或值为 false）
///
/// # 实现细节
///
/// 使用 `tokio::sync::watch::Receiver` 的 `borrow()` 方法获取当前值，
/// 这是一个非阻塞操作，适合在重试循环中频繁检查。
///
/// # 示例
///
/// ```ignore
/// let (tx, rx) = tokio::sync::watch::channel(false);
///
/// assert_eq!(should_abort(Some(&rx)), false);
///
/// tx.send(true).unwrap();
/// assert_eq!(should_abort(Some(&rx)), true);
///
/// assert_eq!(should_abort(None), false);
/// ```
fn should_abort(rx: Option<&tokio::sync::watch::Receiver<bool>>) -> bool {
    rx.is_some_and(|r| *r.borrow())
}

/// 判断助手错误是否可重试
///
/// 该函数分析 [`message::AssistantError`] 的类型，判断是否应该自动重试请求。
/// 只有特定类型的错误被认为是临时的、可通过重试解决的。
///
/// # 参数
///
/// - `err`: 助手错误的引用，来自 API 调用失败
///
/// # 返回值
///
/// - `true`: 错误可重试（如临时网络错误、速率限制等）
/// - `false`: 错误不应重试（如认证失败、参数错误等）
///
/// # 可重试的错误类型
///
/// 目前仅 [`message::AssistantError::APIError`] 类型中 `is_retryable` 字段为 `true` 的错误被认为可重试。
/// 这通常包括：
/// - 网络超时或连接失败
/// - 服务暂时不可用（503）
/// - 速率限制（429）
/// - 提供商内部错误（500）
///
/// # 不可重试的错误类型
///
/// 以下错误不应重试：
/// - 认证失败（401/403）
/// - 请求参数错误（400）
/// - 模型不存在（404）
/// - 内容过滤违规
/// - 消息被用户中断
/// - 其他未知错误类型
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::session::message::AssistantError;
///
/// // 可重试的错误
/// let retryable = AssistantError::APIError {
///     message: "Service Unavailable".into(),
///     is_retryable: true,
/// };
/// assert_eq!(is_retryable_assistant_error(&retryable), true);
///
/// // 不可重试的错误
/// let non_retryable = AssistantError::APIError {
///     message: "Unauthorized".into(),
///     is_retryable: false,
/// };
/// assert_eq!(is_retryable_assistant_error(&non_retryable), false);
///
/// // 其他类型错误
/// let other = AssistantError::MessageAbortedError { message: "aborted".into() };
/// assert_eq!(is_retryable_assistant_error(&other), false);
/// ```
fn is_retryable_assistant_error(err: &message::AssistantError) -> bool {
    match err {
        message::AssistantError::APIError { is_retryable, .. } => *is_retryable,
        message::AssistantError::Unknown { message } if is_acp_session_changed_message(message) => {
            true
        }
        _ => false,
    }
}

fn is_acp_retryable_assistant_error(err: &message::AssistantError) -> bool {
    match err {
        message::AssistantError::Unknown { message } => {
            let normalized = message.to_ascii_lowercase();
            is_acp_session_changed_message(message)
                || normalized.contains("timed out")
                || normalized.contains("timeout")
                || normalized.contains("acp agent disconnected during request")
                || normalized.contains("queue owner disconnected before prompt completion")
                || normalized.contains("error decoding response body")
                || normalized.contains("eof while parsing a value")
        }
        message::AssistantError::APIError { message, .. } => {
            let normalized = message.to_ascii_lowercase();
            is_acp_session_changed_message(message)
                || normalized.contains("timed out")
                || normalized.contains("timeout")
                || normalized.contains("acp agent disconnected during request")
                || normalized.contains("error decoding response body")
                || normalized.contains("eof while parsing a value")
        }
        _ => false,
    }
}

fn is_acp_session_changed_message(message: &str) -> bool {
    message.to_ascii_lowercase().contains("acp session changed:")
}

/// 供应商边界：归一化后不得再出现本地路径或远程 URL 图片标记。
fn assert_no_raw_image_paths(messages: &Value) -> Result<(), String> {
    let Some(arr) = messages.as_array() else {
        return Ok(());
    };
    for message in arr {
        let Some(content) = message.get("content").and_then(Value::as_str) else {
            continue;
        };
        let (_, refs) = crate::app::agent::multimodal::parse_image_markers(content);
        for reference in refs {
            if reference.starts_with("data:") {
                continue;
            }
            return Err(format!(
                "multimodal provider boundary rejected raw image reference (expected data URI or asset text)"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod stream_tests;
