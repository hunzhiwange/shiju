//! 流式请求处理模块
//!
//! 本模块提供基于 aisdk 的流式语言模型请求处理功能。通过异步流式传输，
//! 实时接收并处理模型生成的内容，支持文本生成、推理过程和工具调用。
//!
//! # 主要功能
//!
//! - 流式文本生成：实时接收模型生成的文本内容
//! - 推理过程追踪：捕获模型的推理（reasoning）输出
//! - 工具调用处理：解析和管理模型请求的工具调用
//! - 中止机制：支持通过 watch channel 优雅中止请求
//! - 使用量统计：收集 token 使用情况等统计信息
//!
//! # 平台兼容性
//!
//! 本模块所有功能仅在非 WASM32 目标平台上可用（通过 `#[cfg(not(target_arch = "wasm32"))]` 控制）。
//! 这是因为 aisdk 和 tokio 的异步运行时在 WASM 环境中存在兼容性限制。

#[cfg(not(target_arch = "wasm32"))]
use aisdk::core::LanguageModelRequest;
#[cfg(not(target_arch = "wasm32"))]
use aisdk::core::capabilities::ToolCallSupport;
#[cfg(not(target_arch = "wasm32"))]
use aisdk::core::language_model::LanguageModelResponseContentType as AiContent;
#[cfg(not(target_arch = "wasm32"))]
use aisdk::core::language_model::{
    LanguageModelStreamChunk as AiStreamChunk, LanguageModelStreamChunkType,
};
#[cfg(not(target_arch = "wasm32"))]
use aisdk::core::tools::{Tool as AiTool, ToolExecute as AiToolExecute};
#[cfg(not(target_arch = "wasm32"))]
use futures_util::StreamExt;
#[cfg(not(target_arch = "wasm32"))]
use serde_json::{Map, Value};
#[cfg(not(target_arch = "wasm32"))]
use std::collections::{HashMap, HashSet};

#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::session::llm::logging::LOGGER;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::session::llm::types::{Error, StreamEvent, ToolCall};
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::tools;

#[cfg(not(target_arch = "wasm32"))]
use super::error::{aisdk_assistant_error_log_fields, assistant_error_from_aisdk};
#[cfg(not(target_arch = "wasm32"))]
use super::util::{
    normalize_strict_object_required, should_abort, stop_sequences_from_value,
    token_usage_from_aisdk_usage,
};

#[cfg(not(target_arch = "wasm32"))]
fn aisdk_stream_chunk_error(
    kind: &str,
    message: String,
) -> crate::app::agent::session::message::AssistantError {
    crate::app::agent::session::message::AssistantError::APIError {
        message: message.clone(),
        status_code: None,
        is_retryable: false,
        response_headers: None,
        response_body: Some(message.clone()),
        metadata: Some(HashMap::from([
            ("source".to_string(), "aisdk".to_string()),
            ("raw_error".to_string(), message),
            ("stream_failure_kind".to_string(), kind.to_string()),
        ])),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_reasoning_effort_label(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "low" => Some("low"),
        "medium" => Some("medium"),
        "high" => Some("high"),
        _ => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn uses_dashscope_reasoning_body(provider_id: &str, request_url: &str) -> bool {
    provider_id.eq_ignore_ascii_case("alibaba-cn")
        || request_url.contains("dashscope.aliyuncs.com")
        || request_url.contains("dashscope-us.aliyuncs.com")
        || request_url.contains("dashscope-intl.aliyuncs.com")
}

#[cfg(not(target_arch = "wasm32"))]
fn reasoning_request_config(
    provider_id: &str,
    request_url: &str,
    obj: &Map<String, Value>,
) -> (Option<&'static str>, Option<Map<String, Value>>) {
    let effort =
        obj.get("reasoning_effort").and_then(Value::as_str).and_then(parse_reasoning_effort_label);

    if effort.is_none() {
        return (None, None);
    }

    if uses_dashscope_reasoning_body(provider_id, request_url) {
        let mut body = Map::new();
        body.insert("enable_thinking".to_string(), Value::Bool(true));
        (None, Some(body))
    } else {
        (effort, None)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn normalize_tool_call_id(id: &str, request_namespace: &str) -> String {
    let Some(index) = id.strip_prefix("openai-tool-call-") else {
        return id.to_string();
    };
    if index.is_empty() || !index.bytes().all(|byte| byte.is_ascii_digit()) {
        return id.to_string();
    }

    format!("openai-tool-call-{request_namespace}-{index}")
}

/// 执行带模型的 aisdk 流式请求
///
/// 该函数是 aisdk 流式请求的核心入口点，负责构建请求、发送流式调用、
/// 并通过回调函数实时推送流事件。
///
/// # 类型参数
///
/// - `M`: 语言模型类型，必须实现 `LanguageModel` 和 `ToolCallSupport` trait
///
/// # 参数
///
/// - `provider_id`: 提供商标识符，用于日志记录和错误溯源
/// - `model`: 语言模型实例，承载具体的模型配置和认证信息
/// - `request_url`: 请求的 URL 地址，用于日志记录
/// - `enforce_strict_tool_schema`: 是否强制规范化工具参数 schema
/// - `messages`: 发送给模型的对话消息列表
/// - `tools`: 可用工具的映射表，键为工具名称，值为工具规格
/// - `temperature`: 采样温度（0.0-1.0），控制输出的随机性
/// - `top_p`: 核采样参数（0.0-1.0），控制词汇选择的范围
/// - `max_output_tokens`: 最大输出 token 数量限制
/// - `merged_options`: 合并后的额外选项（JSON 对象），支持多种高级参数
/// - `retries`: 请求失败时的最大重试次数
/// - `abort`: 可选的中止信号接收器，用于优雅取消请求
/// - `on_event`: 流事件回调函数，用于处理每个流事件
///
/// # 返回值
///
/// - `Ok(())`: 流式请求成功完成
/// - `Err(Error)`: 请求失败，可能是 API 错误、中止或其他错误
///
/// # 流事件类型
///
/// 回调函数会接收到以下事件：
/// - `StreamEvent::Delta`: 文本内容增量
/// - `StreamEvent::ReasoningDelta`: 推理过程增量
/// - `StreamEvent::ToolCalls`: 工具调用列表
/// - `StreamEvent::Done`: 流结束，包含完成原因和使用统计
///
/// # 支持的高级选项
///
/// 通过 `merged_options` 参数支持以下字段：
/// - `presence_penalty`: 存在惩罚（f32）
/// - `frequency_penalty`: 频率惩罚（f32）
/// - `seed`: 随机种子（u32）
/// - `top_k`: Top-K 采样参数（u32）
/// - `reasoning_effort`: 推理努力程度（"low"/"medium"/"high"）
/// - `stop_sequences` / `stop`: 停止序列列表
///
/// # 示例
///
/// ```ignore
/// let result = do_stream_request_aisdk_with_model(
///     "openai".to_string(),
///     model,
///     "https://api.openai.com/v1/chat/completions".to_string(),
///     true,
///     messages,
///     &tools,
///     Some(0.7),
///     None,
///     Some(2048),
///     &merged_options,
///     3,
///     Some(&abort_rx),
///     &mut |event| {
///         match event {
///             StreamEvent::Delta(text) => print!("{}", text),
///             StreamEvent::Done { .. } => println!("\n完成"),
///             _ => {}
///         }
///     },
/// ).await;
/// ```
///
/// # 错误处理
///
/// - API 错误：网络问题、认证失败、配额超限等
/// - 中止错误：通过 abort 通道主动取消请求
/// - 流错误：流传输过程中的数据解析或协议错误
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn do_stream_request_aisdk_with_model<
    M: aisdk::core::language_model::LanguageModel + ToolCallSupport,
>(
    provider_id: String,
    model: M,
    request_url: String,
    enforce_strict_tool_schema: bool,
    messages: Vec<aisdk::core::Message>,
    tools: &HashMap<String, tools::ToolSpec>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    max_output_tokens: Option<u64>,
    merged_options: &Value,
    retries: u64,
    abort: Option<&tokio::sync::watch::Receiver<bool>>,
    on_event: &mut (dyn FnMut(StreamEvent) + Send),
) -> Result<(), Error> {
    // 初始化请求构建器，设置模型和消息
    let mut builder = LanguageModelRequest::<M>::builder().model(model).messages(messages);

    // 设置重试次数，限制在 u32 范围内
    builder.max_retries = Some(retries.min(u64::from(u32::MAX)) as u32);

    // 处理工具定义
    if !tools.is_empty() {
        // 对工具键进行排序，确保一致的顺序（便于测试和调试）
        let mut keys = tools.keys().cloned().collect::<Vec<_>>();
        keys.sort();

        for k in keys {
            let Some(spec) = tools.get(&k) else { continue };

            // 克隆参数 schema，必要时进行严格模式规范化
            let mut parameters = spec.input_schema.clone();
            if enforce_strict_tool_schema {
                normalize_strict_object_required(&mut parameters);
            }

            // 将 JSON schema 转换为 aisdk 的 Schema 类型
            let input_schema =
                serde_json::from_value::<aisdk::__private::schemars::Schema>(parameters)
                    .unwrap_or_default();

            // 构建工具定义，execute 函数返回空字符串（实际执行由调用方处理）
            let tool = AiTool {
                name: spec.id.to_string(),
                description: spec.description.to_string(),
                input_schema,
                execute: AiToolExecute::from_sync(|_, _| Ok(String::new())),
            };
            builder = builder.with_tool(tool);
        }
    }

    // 设置温度参数：将 0.0-1.0 映射到 0-100 的整数范围
    if let Some(t) = temperature {
        let t = t.clamp(0.0, 1.0);
        builder.temperature = Some((t * 100.0).round() as u32);
    }

    // 设置 top_p 参数：将 0.0-1.0 映射到 0-100 的整数范围
    if let Some(p) = top_p {
        let p = p.clamp(0.0, 1.0);
        builder.top_p = Some((p * 100.0).round() as u32);
    }

    // 设置最大输出 token 数量
    if let Some(m) = max_output_tokens {
        builder.max_output_tokens = Some(m.min(u64::from(u32::MAX)) as u32);
    }

    // 处理 merged_options 中的高级参数
    if let Some(obj) = merged_options.as_object() {
        // 存在惩罚：控制模型是否倾向于讨论新话题
        if let Some(v) = obj.get("presence_penalty").and_then(Value::as_f64) {
            builder.presence_penalty = Some(v as f32);
        }

        // 频率惩罚：控制模型是否倾向于避免重复相同的词语
        if let Some(v) = obj.get("frequency_penalty").and_then(Value::as_f64) {
            builder.frequency_penalty = Some(v as f32);
        }

        // 随机种子：用于确定性输出
        if let Some(v) = obj.get("seed").and_then(Value::as_u64) {
            builder.seed = Some(v.min(u64::from(u32::MAX)) as u32);
        }

        // Top-K 采样：限制每步只考虑概率最高的 K 个 token
        if let Some(v) = obj.get("top_k").and_then(Value::as_u64) {
            builder.top_k = Some(v.min(u64::from(u32::MAX)) as u32);
        }

        // DashScope 的 OpenAI 兼容 Chat 使用 extra_body.enable_thinking，
        // 不接受顶层 reasoning_effort。
        let (reasoning_effort, reasoning_body) =
            reasoning_request_config(&provider_id, &request_url, obj);
        if let Some(body) = reasoning_body {
            builder = builder.body(Value::Object(body));
        }
        builder.reasoning_effort = reasoning_effort.map(|value| match value {
            "low" => aisdk::core::language_model::ReasoningEffort::Low,
            "medium" => aisdk::core::language_model::ReasoningEffort::Medium,
            "high" => aisdk::core::language_model::ReasoningEffort::High,
            _ => unreachable!("reasoning label should already be normalized"),
        });

        // 停止序列：模型遇到这些序列时会停止生成
        // 优先使用 stop_sequences，回退到 stop
        if let Some(v) = obj.get("stop_sequences") {
            if let Some(seq) = stop_sequences_from_value(v) {
                builder.stop_sequences = Some(seq);
            }
        } else if let Some(v) = obj.get("stop") {
            if let Some(seq) = stop_sequences_from_value(v) {
                builder.stop_sequences = Some(seq);
            }
        }
    }

    // 构建最终请求
    let req = builder.build();

    // 克隆请求选项（不包含模型实例）
    let options = std::ops::Deref::deref(&req).clone();

    // 提取模型实例
    let mut model = req.model;

    // 记录流请求开始日志
    LOGGER.clone_logger().tag("providerID", &provider_id).info(
        "aisdk.stream.start",
        Some({
            let mut m = serde_json::Map::new();
            m.insert("requestURL".to_string(), Value::String(request_url.clone()));
            m
        }),
    );

    // 发起流式请求，处理可能的打开流失败错误
    let mut stream = model.stream_text(options).await.map_err(|e| {
        let assistant_error = assistant_error_from_aisdk(&provider_id, e);
        LOGGER.clone_logger().tag("providerID", &provider_id).error(
            "aisdk.stream.open_failed",
            Some({
                let mut m = aisdk_assistant_error_log_fields(&assistant_error);
                m.insert("requestURL".to_string(), Value::String(request_url.clone()));
                m
            }),
        );
        Error::Api(assistant_error)
    })?;

    // 初始化流处理状态变量
    let mut tool_calls: Vec<ToolCall> = Vec::new(); // 收集工具调用
    let mut tool_call_ids: HashSet<String> = HashSet::new(); // 去重，兼容 Available + Done 双路径
    // OpenAI 兼容接口在缺失 provider call id 时，AI SDK 只用请求内的工具索引兜底。
    // 为该兜底 ID 加上请求命名空间，避免多轮请求都生成 `openai-tool-call-0`。
    let tool_call_request_namespace = uuid::Uuid::new_v4().simple().to_string();
    let mut last_usage: Option<aisdk::core::language_model::Usage> = None; // 最新使用统计
    let mut saw_text = false; // 是否已发送过文本增量
    let mut saw_reasoning = false; // 是否已发送过推理增量

    // 主流处理循环
    loop {
        // 检查是否应中止请求
        if should_abort(abort) {
            return Err(Error::Aborted);
        }

        // 获取下一个流项目，支持中止信号中断
        let next = if let Some(rx) = abort {
            let mut rx = rx.clone();
            tokio::select! {
                // 中止信号发生变化时立即返回中止错误
                _ = rx.changed() => {
                    return Err(Error::Aborted);
                }
                // 正常接收流项目
                item = stream.next() => item,
            }
        } else {
            stream.next().await
        };

        // 流结束，退出循环
        let Some(item) = next else { break };

        // 将流项目转换为 chunk 列表，处理转换错误
        let chunks = match item {
            Ok(chunks) => chunks,
            Err(e) => {
                let assistant_error = assistant_error_from_aisdk(&provider_id, e);
                LOGGER.clone_logger().tag("providerID", &provider_id).error(
                    "aisdk.stream.read_failed",
                    Some({
                        let mut m = aisdk_assistant_error_log_fields(&assistant_error);
                        m.insert("requestURL".to_string(), Value::String(request_url.clone()));
                        m
                    }),
                );
                return Err(Error::Api(assistant_error));
            }
        };

        // 处理每个 chunk
        for chunk in chunks {
            match chunk {
                // 增量数据块
                AiStreamChunk::Delta(delta) => match delta {
                    // 文本/推理边界标记，无需处理
                    LanguageModelStreamChunkType::TextStart
                    | LanguageModelStreamChunkType::TextEnd
                    | LanguageModelStreamChunkType::ReasoningStart
                    | LanguageModelStreamChunkType::ReasoningEnd => {}

                    // 文本增量
                    LanguageModelStreamChunkType::TextDelta(text) => {
                        if !text.is_empty() {
                            saw_text = true;
                            on_event(StreamEvent::Delta(text));
                        }
                    }

                    // 推理增量
                    LanguageModelStreamChunkType::ReasoningDelta(text) => {
                        if !text.is_empty() {
                            saw_reasoning = true;
                            on_event(StreamEvent::ReasoningDelta(text));
                        }
                    }

                    // 工具调用边界和参数增量由上层按完整调用处理
                    LanguageModelStreamChunkType::ToolCallStart(_)
                    | LanguageModelStreamChunkType::ToolCallDelta { .. }
                    | LanguageModelStreamChunkType::ToolCallEnd(_) => {}

                    // 工具调用已完整可用，直接收集
                    LanguageModelStreamChunkType::ToolCallAvailable(info) => {
                        let call_id =
                            normalize_tool_call_id(&info.tool.id, &tool_call_request_namespace);
                        if tool_call_ids.insert(call_id.clone()) {
                            tool_calls.push(ToolCall {
                                id: call_id,
                                name: info.tool.name.clone(),
                                arguments: serde_json::to_string(&info.input)
                                    .unwrap_or_else(|_| "{}".to_string()),
                            });
                        }
                    }

                    // 失败、不完整或不支持的消息：返回错误
                    LanguageModelStreamChunkType::Failed(msg) => {
                        let assistant_error = aisdk_stream_chunk_error("failed", msg);
                        LOGGER.clone_logger().tag("providerID", &provider_id).error(
                            "aisdk.stream.chunk_failed",
                            Some({
                                let mut m = aisdk_assistant_error_log_fields(&assistant_error);
                                m.insert(
                                    "requestURL".to_string(),
                                    Value::String(request_url.clone()),
                                );
                                m
                            }),
                        );
                        return Err(Error::Api(assistant_error));
                    }
                    LanguageModelStreamChunkType::Incomplete(msg) => {
                        let assistant_error = aisdk_stream_chunk_error("incomplete", msg);
                        LOGGER.clone_logger().tag("providerID", &provider_id).error(
                            "aisdk.stream.chunk_incomplete",
                            Some({
                                let mut m = aisdk_assistant_error_log_fields(&assistant_error);
                                m.insert(
                                    "requestURL".to_string(),
                                    Value::String(request_url.clone()),
                                );
                                m
                            }),
                        );
                        return Err(Error::Api(assistant_error));
                    }
                    LanguageModelStreamChunkType::NotSupported(msg) => {
                        let assistant_error = aisdk_stream_chunk_error("not_supported", msg);
                        LOGGER.clone_logger().tag("providerID", &provider_id).error(
                            "aisdk.stream.chunk_not_supported",
                            Some({
                                let mut m = aisdk_assistant_error_log_fields(&assistant_error);
                                m.insert(
                                    "requestURL".to_string(),
                                    Value::String(request_url.clone()),
                                );
                                m
                            }),
                        );
                        return Err(Error::Api(assistant_error));
                    }
                },

                // 完成的消息块
                AiStreamChunk::Done(msg) => {
                    // 更新使用统计
                    if let Some(u) = msg.usage.as_ref() {
                        last_usage = Some(u.to_owned());
                    }

                    // 根据内容类型处理
                    match msg.content {
                        // 工具调用：添加到工具调用列表
                        AiContent::ToolCall(info) => {
                            let call_id =
                                normalize_tool_call_id(&info.tool.id, &tool_call_request_namespace);
                            if tool_call_ids.insert(call_id.clone()) {
                                tool_calls.push(ToolCall {
                                    id: call_id,
                                    name: info.tool.name.clone(),
                                    arguments: serde_json::to_string(&info.input)
                                        .unwrap_or_else(|_| "{}".to_string()),
                                });
                            }
                        }

                        // 文本内容：如果之前没有发送过增量，现在发送完整文本
                        AiContent::Text(text) => {
                            if !saw_text && !text.is_empty() {
                                saw_text = true;
                                on_event(StreamEvent::Delta(text));
                            }
                        }

                        // 推理内容：如果之前没有发送过增量，现在发送完整推理
                        AiContent::Reasoning { content, .. } => {
                            if !saw_reasoning && !content.is_empty() {
                                saw_reasoning = true;
                                on_event(StreamEvent::ReasoningDelta(content));
                            }
                        }

                        // 其他内容类型暂不处理
                        _ => {}
                    }
                }
            }
        }
    }

    // 流处理完成，发送最终事件

    // 如果有工具调用，发送工具调用事件
    let has_tool_calls = !tool_calls.is_empty();
    if has_tool_calls {
        on_event(StreamEvent::ToolCalls(tool_calls));
    }

    // 转换使用统计格式
    let usage = last_usage.as_ref().map(token_usage_from_aisdk_usage).unwrap_or_default();

    // 确定完成原因：有工具调用则为 "tool_calls"，否则为 "stop"
    let finish_reason =
        if has_tool_calls { Some("tool_calls".to_string()) } else { Some("stop".to_string()) };

    // 发送完成事件
    on_event(StreamEvent::Done { finish_reason, usage });

    Ok(())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "stream_tests.rs"]
mod stream_tests;
