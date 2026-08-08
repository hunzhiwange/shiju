//! LLM 提示处理模块
//!
//! 本模块提供了与大语言模型（LLM）交互的高级接口，用于发送提示（prompt）并处理流式响应。
//! 主要功能包括：
//!
//! - **模型解析**：支持通过 `provider/model` 格式或纯模型 ID 指定要使用的模型
//! - **流式提示**：支持实时接收模型输出的增量内容（delta）
//! - **同步提示**：等待模型完成响应后返回完整结果
//! - **工具集成**：支持在提示中携带工具定义，启用函数调用能力
//!
//! ## 架构位置
//!
//! 本模块位于会话层（session）与 LLM 层之间，是面向上层业务的高便利接口。
//! 内部依赖 `super::stream` 完成实际的流式对话处理。
//!
//! ## 使用示例
//!
//! ```ignore
//! use std::collections::HashMap;
//!
//! // 流式提示
//! stream_prompt("解释 Rust 的所有权系统", None, |event| {
//!     match event {
//!         PromptStreamEvent::Delta(text) => print!("{}", text),
//!         PromptStreamEvent::Done(usage) => println!("\nToken 使用：{:?}", usage),
//!         PromptStreamEvent::Error(e) => eprintln!("错误：{}", e),
//!     }
//! });
//!
//! // 同步提示
//! let (usage, response) = send_prompt("什么是闭包？", Some("openai/gpt-4".to_string()))?;
//! println!("回答：{}\nToken 使用：{:?}", response, usage);
//! ```

use crate::app::agent::provider::provider;
use crate::app::agent::session::message;
use crate::app::agent::tools;
use crate::session::ui_types as models;
use serde_json::{Value, json};
use std::collections::HashMap;

use super::types::{AgentInfo, PromptStreamEvent, StreamEvent, StreamInput};

/// 解析模型标识符并获取对应的模型配置
///
/// 该函数支持多种模型指定方式，并自动处理歧义情况：
///
/// ## 参数
///
/// - `model`: 可选的模型标识符字符串，支持以下格式：
///   - `None` 或空字符串：使用系统默认模型
///   - `"model-id"`：纯模型 ID，将在所有 provider 中搜索
///   - `"provider-id/model-id"`：明确指定 provider 和模型
///
/// ## 返回值
///
/// - `Ok(provider::Model)`: 成功找到的模型配置
/// - `Err(String)`: 错误信息，可能的原因包括：
///   - 模型格式错误（provider 或 model 部分为空）
///   - 未找到指定模型
///   - 模型 ID 存在歧义（多个 provider 都有同名模型）
///
/// ## 模型解析逻辑
///
/// 1. 如果输入为空或 `None`，返回系统默认模型
/// 2. 如果输入包含 `/`，按 `provider/model` 格式解析
/// 3. 否则，在所有 provider 中搜索该模型 ID
///    - 如果唯一匹配，直接返回
///    - 如果无匹配，返回错误
///    - 如果多处匹配，返回歧义错误并列出所有候选项
///
/// ## 示例
///
/// ```ignore
/// // 使用默认模型
/// let model = resolve_model(None).await?;
///
/// // 使用指定模型
/// let model = resolve_model(Some("gpt-4".to_string())).await?;
///
/// // 明确指定 provider
/// let model = resolve_model(Some("openai/gpt-4".to_string())).await?;
/// ```
async fn resolve_model(model: Option<String>) -> Result<provider::Model, String> {
    // 处理用户提供的模型标识符：去除空白并检查是否非空
    if let Some(s) = model.as_deref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
        // 获取所有可用的 provider 列表
        let providers = provider::list().await;

        // 如果标识符包含 `/`，按 provider/model 格式解析
        if s.contains('/') {
            let parsed = provider::parse_model(&s);

            // 验证解析结果的完整性
            if parsed.provider_id.trim().is_empty() || parsed.model_id.trim().is_empty() {
                return Err(format!("模型格式错误：{}，请使用 provider/model", s));
            }

            // 直接获取指定的模型
            return provider::get_model(&parsed.provider_id, &parsed.model_id)
                .await
                .map_err(|e| e.to_string());
        }

        // 纯模型 ID 模式：在所有 provider 中搜索
        let mut candidates = Vec::<String>::new();
        for (provider_id, info) in &providers {
            if info.models.contains_key(&s) {
                candidates.push(provider_id.clone());
            }
        }

        // 无匹配结果
        if candidates.is_empty() {
            return Err(format!("未找到模型：{}", s));
        }

        // 存在歧义：多个 provider 都有同名模型
        if candidates.len() > 1 {
            candidates.sort();
            // 构造所有可能的完整标识符列表
            let options = candidates
                .iter()
                .map(|provider_id| format!("{}/{}", provider_id, s))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!(
                "模型ID存在歧义：{}，请使用 provider/model（可选：{}）",
                s, options
            ));
        }

        // 唯一匹配，返回该模型
        return provider::get_model(&candidates[0], &s).await.map_err(|e| e.to_string());
    }

    // 未提供模型标识符，使用系统默认模型
    let parsed = provider::default_model().await.map_err(|e| e.to_string())?;
    provider::get_model(&parsed.provider_id, &parsed.model_id).await.map_err(|e| e.to_string())
}

fn request_options_enable_acp(options: &Value) -> bool {
    options.get("acp_test").and_then(Value::as_bool).unwrap_or(false)
        || options
            .get("acp_agent")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
}

fn acp_route_model(options: &Value) -> provider::Model {
    let model_id = options
        .get("acp_agent")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("acp")
        .to_string();
    provider::Model {
        id: model_id.clone(),
        provider_id: "acp".to_string(),
        api: provider::ApiInfo {
            id: model_id.clone(),
            url: String::new(),
            adapter: "acp".to_string(),
        },
        name: model_id,
        family: None,
        capabilities: provider::Capabilities {
            temperature: false,
            reasoning: false,
            attachment: false,
            toolcall: true,
            input: provider::CapabilityIO {
                text: true,
                audio: false,
                image: false,
                video: false,
                pdf: false,
            },
            output: provider::CapabilityIO {
                text: true,
                audio: false,
                image: false,
                video: false,
                pdf: false,
            },
            interleaved: provider::InterleavedCapability::Bool(false),
        },
        cost: provider::ModelCost {
            input: 0.0,
            output: 0.0,
            cache: provider::ModelCostCache { read: 0.0, write: 0.0 },
            experimental_over_200k: None,
        },
        limit: provider::ModelLimit { context: 8192, input: None, output: 4096 },
        status: "active".to_string(),
        options: HashMap::new(),
        headers: HashMap::new(),
        release_date: "1970-01-01".to_string(),
        variants: HashMap::new(),
    }
}

async fn resolve_model_for_options(
    model: Option<String>,
    options: &Value,
) -> Result<provider::Model, String> {
    let has_explicit_model = model.as_deref().map(str::trim).is_some_and(|value| !value.is_empty());
    if !has_explicit_model && request_options_enable_acp(options) {
        return Ok(acp_route_model(options));
    }

    resolve_model(model).await
}

/// 在请求前读取与实际 LLM 路由相同的模型限制，不发起模型请求。
pub(crate) fn model_limit_for_request(
    model: Option<String>,
    options: &Value,
) -> provider::ModelLimit {
    block_on(async {
        resolve_model_for_options(model, options)
            .await
            .map(|resolved| resolved.limit)
            .unwrap_or_else(|_| provider::resolve_model_limit(None, None, None))
    })
}

/// 获取当前时间的 Unix 时间戳（毫秒）
///
/// ## 返回值
///
/// 返回自 Unix 纪元（1970-01-01 00:00:00 UTC）以来经过的毫秒数。
/// 如果系统时间获取失败，返回 0。
///
/// ## 平台兼容性
///
/// 使用 `web_time` crate 确保在 WebAssembly 和原生平台上都能正常工作。
fn now_ms() -> u64 {
    web_time::SystemTime::now()
        .duration_since(web_time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// 创建用于提示接口的虚拟用户信息
///
/// 该函数构造一个 `UserInfo` 结构体，用于在简化的提示接口中
/// 模拟用户会话上下文。生成的用户信息包含必要的元数据，
/// 但省略了完整会话中才需要的字段（如摘要、系统提示等）。
///
/// ## 参数
///
/// - `session_id`: 会话标识符，用于关联这次提示
/// - `model`: 使用的模型配置，从中提取 provider 和 model ID
///
/// ## 返回值
///
/// 返回一个 `message::UserInfo` 实例，包含：
/// - 自动生成的唯一 ID（格式：`prompt-{timestamp}`）
/// - 当前时间戳
/// - 构建模式（`build`）的代理标识
/// - 模型引用信息
fn dummy_user(session_id: &str, model: &provider::Model) -> message::UserInfo {
    message::UserInfo {
        id: format!("prompt-{}", now_ms()),
        session_id: session_id.to_string(),
        time: message::UserTime { created: now_ms() },
        summary: None,
        agent: "build".to_string(),
        model: message::ModelRef {
            provider_id: model.provider_id.clone(),
            model_id: model.id.clone(),
        },
        system: None,
        tools: None,
        variant: None,
    }
}

/// 在同步上下文中执行异步 Future
///
/// 该函数提供了一个在同步代码中执行异步操作的桥梁。
/// 它会智能检测当前的运行时环境：
///
/// - 如果已经在 Tokio 运行时中，使用 `block_in_place` 避免阻塞整个运行时
/// - 如果不在任何运行时中，创建一个临时的单线程运行时
///
/// ## 类型参数
///
/// - `F`: 要执行的 Future 类型
///
/// ## 参数
///
/// - `fut`: 要执行的异步 Future
///
/// ## 返回值
///
/// 返回 Future 的输出结果
///
/// ## 平台限制
///
/// - **WASM**：不支持此函数，调用将触发 panic
/// - **原生平台**：完全支持
///
/// ## Panics
///
/// - 在 WASM 目标上调用时立即 panic
/// - 创建临时运行时失败时 panic
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    // WASM 平台不支持阻塞操作
    #[cfg(target_arch = "wasm32")]
    panic!("block_on not supported on WASM");

    #[cfg(not(target_arch = "wasm32"))]
    {
        // 尝试获取当前 Tokio 运行时的句柄
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            // 在现有运行时中使用 block_in_place 避免阻塞整个运行时
            return tokio::task::block_in_place(|| handle.block_on(fut));
        }

        // 不在任何运行时中，创建临时单线程运行时
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime")
            .block_on(fut)
    }
}

/// 流式发送提示并实时接收响应
///
/// 该函数将用户提示发送给 LLM，并通过回调函数实时返回模型的增量输出。
/// 适用于需要逐字显示响应的场景（如聊天界面、实时输出等）。
///
/// ## 参数
///
/// - `prompt`: 用户输入的提示文本
/// - `model`: 可选的模型标识符（格式见 [`resolve_model`] 的说明）
///   - `None`: 使用系统默认模型
///   - `Some("model-id")`: 使用指定模型
///   - `Some("provider/model")`: 使用指定 provider 的指定模型
/// - `on_event`: 事件回调函数，接收 [`PromptStreamEvent`] 类型的流式事件
///
/// ## 事件类型
///
/// 回调函数可能接收到以下事件：
///
/// - `PromptStreamEvent::Delta(String)`: 增量文本输出，应追加显示
/// - `PromptStreamEvent::Done(TokenUsage)`: 流式传输完成，包含 token 使用统计
/// - `PromptStreamEvent::Error(String)`: 发生错误，包含错误描述
///
/// ## 平台限制
///
/// - **Web (WASM)**: 不支持此函数，将立即通过回调返回错误事件
/// - **原生平台**: 完全支持
///
/// ## 使用示例
///
/// ```ignore
/// stream_prompt("解释 Rust 的生命周期", None, |event| {
///     match event {
///         PromptStreamEvent::Delta(text) => {
///             print!("{}", text);
///             // 可以在这里刷新 UI 显示
///         }
///         PromptStreamEvent::Done(usage) => {
///             println!("\n完成！Token 使用：prompt={}, completion={}",
///                 usage.prompt_tokens, usage.completion_tokens);
///         }
///         PromptStreamEvent::Error(e) => {
///             eprintln!("发生错误：{}", e);
///         }
///     }
/// });
/// ```
///
/// ## 内部实现
///
/// 1. 解析模型标识符
/// 2. 构造虚拟用户信息和会话上下文
/// 3. 创建包含用户消息的输入结构
/// 4. 调用底层流式接口，过滤并转发事件
pub fn stream_prompt(
    prompt: &str,
    model: Option<String>,
    mut on_event: impl FnMut(PromptStreamEvent) + Send + 'static,
) {
    // Web 平台不支持此功能
    if cfg!(target_arch = "wasm32") {
        on_event(PromptStreamEvent::Error("Not supported on Web".to_string()));
        return;
    }

    let prompt = prompt.to_string();

    // 解析模型标识符，失败时通过回调返回错误
    let model = match block_on(resolve_model(model)) {
        Ok(m) => m,
        Err(e) => {
            on_event(PromptStreamEvent::Error(e));
            return;
        }
    };

    // 在异步上下文中执行流式请求
    block_on(async move {
        // 构造流式输入结构
        let input = StreamInput {
            user: dummy_user("prompt", &model),
            session_id: "prompt".to_string(),
            model,
            agent: AgentInfo {
                name: "build".to_string(),
                mode: "build".to_string(),
                prompt: Some(String::new()),
                temperature: None,
                top_p: None,
                options: HashMap::new(),
                permission: Default::default(),
            },
            system: Vec::new(),
            abort: None,
            messages: vec![json!({ "role": "user", "content": prompt })],
            small: false,
            tools: HashMap::new(),
            retries: 0,
        };

        let on_event = std::sync::Arc::new(std::sync::Mutex::new(on_event));
        let stream_on_event = on_event.clone();

        // 调用底层流式接口，过滤并转换事件
        let result = super::stream(input, move |ev| match ev {
            // 文本增量：转发给回调
            StreamEvent::Delta(d) => {
                if let Ok(mut on_event) = stream_on_event.lock() {
                    (*on_event)(PromptStreamEvent::Delta(d));
                }
            }
            // 推理增量：在简化接口中忽略
            StreamEvent::ReasoningDelta(_) => {}
            // 工具调用：在简化接口中忽略
            StreamEvent::ToolCalls(_) => {}
            // 完整消息：在简化接口中忽略
            StreamEvent::FullMessages(_) => {}
            // 完成：提取使用统计并转发
            StreamEvent::Done { usage, .. } => {
                if let Ok(mut on_event) = stream_on_event.lock() {
                    (*on_event)(PromptStreamEvent::Done(usage));
                }
            }
            // 错误：转换为用户友好的错误信息
            StreamEvent::Error(e) => {
                if let Ok(mut on_event) = stream_on_event.lock() {
                    (*on_event)(PromptStreamEvent::Error(assistant_error_to_string(&e)));
                }
            }
        })
        .await;

        if let Some(message) = missing_stream_error_message(&result) {
            if let Ok(mut on_event) = on_event.lock() {
                (*on_event)(PromptStreamEvent::Error(message));
            }
        }
    });
}

/// 流式发送带工具定义的提示
///
/// 与 [`stream_prompt`] 类似，但支持携带工具定义，使 LLM 能够进行函数调用。
/// 工具定义遵循模型提供商的规范，允许 LLM 在需要时请求执行特定操作。
///
/// ## 参数
///
/// - `prompt`: 用户输入的提示文本
/// - `model`: 可选的模型标识符（见 [`resolve_model`] 的格式说明）
/// - `tools`: 工具定义映射，键为工具名称，值为工具规格定义
/// - `on_event`: 事件回调函数，接收完整的 [`StreamEvent`] 类型
///
/// ## 事件类型
///
/// 与 [`stream_prompt`] 不同，此函数返回完整的 [`StreamEvent`]，
/// 包括工具调用事件：
///
/// - `StreamEvent::Delta(String)`: 增量文本输出
/// - `StreamEvent::ReasoningDelta(String)`: 推理过程增量（如思维链）
/// - `StreamEvent::ToolCalls(...)`: 工具调用请求
/// - `StreamEvent::FullMessages(...)`: 完整消息列表
/// - `StreamEvent::Done { ... }`: 完成事件
/// - `StreamEvent::Error(AssistantError)`: 错误事件
///
/// ## 平台限制
///
/// - **Web (WASM)**: 不支持
/// - **原生平台**: 完全支持
///
/// ## 使用示例
///
/// ```ignore
/// use std::collections::HashMap;
///
/// let mut tools = HashMap::new();
/// tools.insert(
///     "search".to_string(),
///     ToolSpec::new("search", "搜索工具", serde_json::json!({ "type": "object" })),
/// );
///
/// stream_prompt_with_tools("搜索 Rust 异步编程的最佳实践", None, tools, |event| {
///     match event {
///         StreamEvent::Delta(text) => print!("{}", text),
///         StreamEvent::ToolCalls(calls) => {
///             // 处理工具调用请求
///             for call in calls {
///                 println!("需要调用工具: {}", call.name);
///             }
///         }
///         _ => {}
///     }
/// });
/// ```
///
/// ## 与 stream_prompt 的区别
///
/// | 特性 | stream_prompt | stream_prompt_with_tools |
/// |------|---------------|-------------------------|
/// | 事件类型 | 简化的 PromptStreamEvent | 完整的 StreamEvent |
/// | 工具支持 | 否 | 是 |
/// | 推理输出 | 忽略 | 可接收 |
/// | 适用场景 | 简单对话 | 需要工具调用的复杂场景 |
pub fn stream_prompt_with_tools(
    prompt: &str,
    model: Option<String>,
    tools: HashMap<String, tools::ToolSpec>,
    mut on_event: impl FnMut(StreamEvent) + Send + 'static,
) {
    // Web 平台不支持此功能
    if cfg!(target_arch = "wasm32") {
        (&mut on_event)(StreamEvent::Error(message::AssistantError::Unknown {
            message: "Not supported on Web".to_string(),
        }));
        return;
    }

    let prompt = prompt.to_string();

    // 解析模型标识符
    let model = match block_on(resolve_model(model)) {
        Ok(m) => m,
        Err(e) => {
            (&mut on_event)(StreamEvent::Error(message::AssistantError::Unknown { message: e }));
            return;
        }
    };

    // 执行流式请求
    block_on(async move {
        let input = StreamInput {
            user: dummy_user("prompt", &model),
            session_id: "prompt".to_string(),
            model,
            agent: AgentInfo {
                name: "build".to_string(),
                mode: "build".to_string(),
                prompt: Some(String::new()),
                temperature: None,
                top_p: None,
                options: HashMap::new(),
                permission: Default::default(),
            },
            system: Vec::new(),
            abort: None,
            messages: vec![json!({ "role": "user", "content": prompt })],
            small: false,
            tools,
            retries: 0,
        };

        let on_event = std::sync::Arc::new(std::sync::Mutex::new(on_event));
        let stream_on_event = on_event.clone();

        // 直接转发所有事件
        let result = super::stream(input, move |ev| {
            if let Ok(mut on_event) = stream_on_event.lock() {
                (*on_event)(ev);
            }
        })
        .await;
        if let Some(message) = missing_stream_error_message(&result) {
            if let Ok(mut on_event) = on_event.lock() {
                (*on_event)(StreamEvent::Error(message::AssistantError::Unknown { message }));
            }
        }
    });
}

/// 流式发送多轮对话消息（带工具支持）
///
/// 该函数支持完整的多轮对话场景，允许传入消息历史、系统提示和工具定义。
/// 与单轮提示函数不同，此函数接受完整的消息列表，适用于需要维护上下文的对话场景。
///
/// ## 参数
///
/// - `messages`: 对话消息列表，每条消息是一个 JSON 对象，包含 `role` 和 `content` 字段
///   - `role`: 可以是 `"user"`、`"assistant"` 或 `"system"`
///   - `content`: 消息内容
/// - `system`: 系统提示列表，用于设置 LLM 的行为和角色
/// - `model`: 可选的模型标识符（见 [`resolve_model`] 的格式说明）
/// - `tools`: 工具定义映射
/// - `on_event`: 事件回调函数
///
/// ## 消息格式示例
///
/// ```ignore
/// let messages = vec![
///     json!({ "role": "user", "content": "你好！" }),
///     json!({ "role": "assistant", "content": "你好！有什么我可以帮助你的吗？" }),
///     json!({ "role": "user", "content": "请解释一下 Rust 的所有权" }),
/// ];
///
/// let system = vec![
///     "你是一个 Rust 编程专家".to_string(),
///     "回答要简洁明了".to_string(),
/// ];
///
/// stream_chat_with_tools(messages, system, None, HashMap::new(), |event| {
///     // 处理事件...
/// });
/// ```
///
/// ## 平台限制
///
/// - **Web (WASM)**: 不支持
/// - **原生平台**: 完全支持
///
/// ## 与其他函数的对比
///
/// | 函数 | 消息类型 | 系统提示 | 工具支持 |
/// |------|---------|---------|---------|
/// | stream_prompt | 单条 | 否 | 否 |
/// | stream_prompt_with_tools | 单条 | 否 | 是 |
/// | stream_chat_with_tools | 多条 | 是 | 是 |
/// | send_prompt | 单条 | 否 | 否 |
///
/// ## 使用场景
///
/// - 需要维护对话历史的聊天应用
/// - 需要自定义系统提示的场景
/// - 多轮对话中的工具调用
pub fn stream_chat_with_tools(
    messages: Vec<Value>,
    system: Vec<String>,
    model: Option<String>,
    options: Value,
    tools: HashMap<String, tools::ToolSpec>,
    mut on_event: impl FnMut(StreamEvent) + Send + 'static,
) {
    stream_chat_with_tools_for_session(
        "chat",
        messages,
        system,
        model,
        options,
        tools,
        move |ev| (&mut on_event)(ev),
    );
}

pub fn stream_chat_with_tools_for_session(
    session_id: &str,
    messages: Vec<Value>,
    system: Vec<String>,
    model: Option<String>,
    options: Value,
    tools: HashMap<String, tools::ToolSpec>,
    mut on_event: impl FnMut(StreamEvent) + Send + 'static,
) {
    stream_chat_with_tools_for_session_with_abort(
        session_id,
        messages,
        system,
        model,
        options,
        tools,
        None,
        move |ev| (&mut on_event)(ev),
    );
}

pub(crate) fn stream_chat_with_tools_for_session_with_abort(
    session_id: &str,
    messages: Vec<Value>,
    system: Vec<String>,
    model: Option<String>,
    options: Value,
    tools: HashMap<String, tools::ToolSpec>,
    abort: Option<tokio::sync::watch::Receiver<bool>>,
    mut on_event: impl FnMut(StreamEvent) + Send + 'static,
) {
    // Web 平台不支持此功能
    if cfg!(target_arch = "wasm32") {
        (&mut on_event)(StreamEvent::Error(message::AssistantError::Unknown {
            message: "Not supported on Web".to_string(),
        }));
        return;
    }

    let session_id = session_id.to_string();

    // 解析模型标识符
    let model = match block_on(resolve_model_for_options(model, &options)) {
        Ok(m) => m,
        Err(e) => {
            (&mut on_event)(StreamEvent::Error(message::AssistantError::Unknown { message: e }));
            return;
        }
    };

    // 执行流式请求
    block_on(async move {
        let agent_options = options
            .as_object()
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        let input = StreamInput {
            user: dummy_user(&session_id, &model),
            session_id: session_id.clone(),
            model,
            agent: AgentInfo {
                name: "build".to_string(),
                mode: "build".to_string(),
                prompt: Some(String::new()),
                temperature: None,
                top_p: None,
                options: agent_options,
                permission: Default::default(),
            },
            system,
            abort,
            messages,
            small: false,
            tools,
            retries: 0,
        };

        let on_event = std::sync::Arc::new(std::sync::Mutex::new(on_event));
        let stream_on_event = on_event.clone();

        // 直接转发所有事件
        let result = super::stream(input, move |ev| {
            if let Ok(mut on_event) = stream_on_event.lock() {
                (*on_event)(ev);
            }
        })
        .await;
        if let Some(message) = missing_stream_error_message(&result) {
            if let Ok(mut on_event) = on_event.lock() {
                (*on_event)(StreamEvent::Error(message::AssistantError::Unknown { message }));
            }
        }
    });
}

/// 同步发送提示并获取完整响应
///
/// 与流式函数不同，此函数会等待 LLM 完成所有输出后返回完整结果。
/// 适用于不需要实时显示、只需要最终结果的场景。
///
/// ## 参数
///
/// - `prompt`: 用户输入的提示文本
/// - `model`: 可选的模型标识符（见 [`resolve_model`] 的格式说明）
///
/// ## 返回值
///
/// - `Ok((TokenUsage, String))`: 成功时返回元组
///   - `TokenUsage`: Token 使用统计（包含 prompt_tokens 和 completion_tokens）
///   - `String`: 模型的完整响应文本
/// - `Err(String)`: 错误信息
///
/// ## 平台限制
///
/// - **Web (WASM)**: 不支持，返回错误
/// - **原生平台**: 完全支持
///
/// ## 使用示例
///
/// ```ignore
/// // 使用默认模型
/// match send_prompt("什么是闭包？", None) {
///     Ok((usage, response)) => {
///         println!("回答：{}", response);
///         println!("Token 使用：prompt={}, completion={}",
///             usage.prompt_tokens, usage.completion_tokens);
///     }
///     Err(e) => eprintln!("错误：{}", e),
/// }
///
/// // 使用指定模型
/// let (usage, response) = send_prompt(
///     "解释所有权系统",
///     Some("openai/gpt-4".to_string())
/// )?;
/// ```
///
/// ## 性能考虑
///
/// 由于此函数会阻塞直到 LLM 完成响应，在以下场景中应考虑使用流式接口：
///
/// - 需要实时显示响应的交互式应用
/// - 响应可能很长的复杂查询
/// - 需要更好的用户体验的场景
///
/// ## 内部实现
///
/// 函数内部使用 `Arc<Mutex>` 在同步和异步边界之间共享状态，
/// 收集所有增量输出直到完成或出错。
pub fn send_prompt(
    prompt: &str,
    model: Option<String>,
) -> Result<(models::TokenUsage, String), String> {
    // Web 平台不支持此功能
    if cfg!(target_arch = "wasm32") {
        return Err("Not supported on Web".to_string());
    }

    let prompt = prompt.to_string();

    block_on(async move {
        // 解析模型
        let model = resolve_model(model).await?;

        // 构造输入
        let input = StreamInput {
            user: dummy_user("prompt", &model),
            session_id: "prompt".to_string(),
            model,
            agent: AgentInfo {
                name: "build".to_string(),
                mode: "build".to_string(),
                prompt: Some(String::new()),
                temperature: None,
                top_p: None,
                options: HashMap::new(),
                permission: Default::default(),
            },
            system: Vec::new(),
            abort: None,
            messages: vec![json!({ "role": "user", "content": prompt })],
            small: false,
            tools: HashMap::new(),
            retries: 0,
        };

        // 使用 Arc<Mutex> 共享状态来收集流式输出
        // 元组内容：(token 使用统计, 累积的响应文本, 可选的错误信息)
        let state = std::sync::Arc::new(std::sync::Mutex::new((
            models::TokenUsage::default(),
            String::new(),
            Option::<String>::None,
        )));

        let state2 = state.clone();

        // 执行流式请求并收集结果
        super::stream(input, move |ev| {
            let mut lock = state2.lock().unwrap_or_else(|e| e.into_inner());

            match ev {
                // 累积文本增量
                StreamEvent::Delta(d) => lock.1.push_str(&d),
                // 推理增量：忽略
                StreamEvent::ReasoningDelta(_) => {}
                // 工具调用：忽略
                StreamEvent::ToolCalls(_) => {}
                // 完整消息：忽略
                StreamEvent::FullMessages(_) => {}
                // 记录 token 使用统计
                StreamEvent::Done { usage, .. } => lock.0 = usage,
                // 记录错误
                StreamEvent::Error(e) => lock.2 = Some(assistant_error_to_string(&e)),
            }
        })
        .await
        .map_err(|e| e.to_string())?;

        // 提取最终结果
        let (usage, out, err) = state.lock().unwrap_or_else(|e| e.into_inner()).clone();

        // 如果有错误，返回错误
        if let Some(err) = err {
            return Err(err);
        }

        Ok((usage, out))
    })
}

/// 将 AssistantError 转换为用户友好的错误信息
///
/// 该函数将内部的 `AssistantError` 枚举转换为面向用户的中文错误描述，
/// 提供更友好、更具可读性的错误信息。
///
/// ## 参数
///
/// - `e`: 助手错误的引用
///
/// ## 返回值
///
/// 返回用户友好的中文错误描述字符串
///
/// ## 错误类型映射
///
/// | 错误类型 | 中文描述 |
/// |---------|---------|
/// | ProviderAuthError | 未配置 {provider} 的 API Key：{message} |
/// | MessageOutputLengthError | 模型输出过长 |
/// | MessageAbortedError | {message} |
/// | ContextOverflowError | {message} |
/// | APIError | {message} |
/// | Unknown | {message} |
///
/// ## 使用示例
///
/// ```ignore
/// let error = AssistantError::ProviderAuthError {
///     provider_id: "openai".to_string(),
///     message: "请设置 OPENAI_API_KEY".to_string(),
/// };
/// let user_message = assistant_error_to_string(&error);
/// // 结果: "未配置 openai 的 API Key：请设置 OPENAI_API_KEY"
/// ```
fn assistant_error_to_string(e: &message::AssistantError) -> String {
    match e {
        // Provider 认证错误：缺少 API Key
        message::AssistantError::ProviderAuthError { provider_id, message } => {
            format!("未配置 {} 的 API Key：{}", provider_id, message)
        }
        // 输出长度超限
        message::AssistantError::MessageOutputLengthError => "模型输出过长".to_string(),
        // 消息被中止
        message::AssistantError::MessageAbortedError { message } => message.to_string(),
        // 上下文溢出（token 超过限制）
        message::AssistantError::ContextOverflowError { message, .. } => message.to_string(),
        // API 调用错误
        message::AssistantError::APIError { message, .. } => message.to_string(),
        // 未知错误
        message::AssistantError::Unknown { message } => message.to_string(),
    }
}

fn missing_stream_error_message(result: &Result<(), super::Error>) -> Option<String> {
    match result {
        Ok(()) | Err(super::Error::Api(_)) | Err(super::Error::Aborted) => None,
        Err(super::Error::ProviderNotFound(provider_id)) => {
            Some(format!("未找到 provider：{}", provider_id))
        }
        Err(super::Error::Http(error)) => Some(error.to_string()),
    }
}
#[cfg(test)]
#[path = "prompt_tests.rs"]
mod prompt_tests;
