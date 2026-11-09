//! 会话标题生成模块
//!
//! 本模块提供基于会话内容自动生成标题的功能。通过调用大语言模型（LLM），
//! 根据用户的对话内容生成简洁、准确的会话标题，便于会话管理和检索。
//!
//! # 主要功能
//!
//! - 基于会话首条消息内容生成标题
//! - 支持指定模型或复用 auto 模型选择策略
//! - 自动截断过长的标题，保证标题简洁性
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use crate::app::agent::session::title::generate_from_content;
//!
//! let title = generate_from_content(
//!     "session-123".to_string(),
//!     "如何学习 Rust 编程语言？".to_string(),
//!     Some("anthropic/claude-3".to_string()),
//! ).await?;
//! println!("生成的标题: {}", title);
//! ```

use crate::app::agent::provider::provider;
use serde_json::Value;
use std::collections::HashMap;

/// 标题最大字符数
const MAX_TITLE_CHARS: usize = 50;

/// 用于生成标题的内容截断长度
const CONTENT_TRUNCATE_CHARS: usize = 200;

/// 标题截断后省略号前的字符数
const TITLE_TRUNCATE_KEEP: usize = 47;

/// 内部标题请求的固定运行模式。
const INTERNAL_TITLE_MODE: &str = "chat";

fn truncate_title_source(content: String) -> String {
    if content.chars().count() > CONTENT_TRUNCATE_CHARS {
        content.chars().take(CONTENT_TRUNCATE_CHARS).collect::<String>()
    } else {
        content
    }
}

/// Extract display text from a successful user turn without exposing attachment payloads.
pub(crate) fn first_response_source(content: &str) -> String {
    use vw_shared::session::session_utils::strip_task_session_source_marker;

    let content = strip_task_session_source_marker(content);
    let text = match serde_json::from_str::<Vec<Value>>(&content) {
        Ok(blocks)
            if !blocks.is_empty()
                && blocks.iter().all(|block| {
                    matches!(
                        block["type"].as_str(),
                        Some(
                            "text" | "image" | "image_url" | "resource" | "resource_link" | "audio"
                        )
                    )
                }) =>
        {
            blocks
                .iter()
                .filter(|block| block["type"] == "text")
                .filter_map(|block| block["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n")
        }
        _ => content.into_owned(),
    };
    let (text, _) = crate::multimodal::parse_image_markers(&text);
    let text = text
        .lines()
        .filter(|line| !line.trim().starts_with("[Resource: "))
        .collect::<Vec<_>>()
        .join("\n");
    truncate_title_source(strip_task_session_source_marker(&text).trim().to_owned())
}

fn build_title_prompt(content: &str) -> String {
    format!(
        "Generate a concise title (max 50 characters) for this conversation. Only output the title, nothing else.\n\nUser message:\n{}",
        content
    )
}

fn apply_acp_agent_options(options: &mut HashMap<String, Value>, acp_agent: Option<&str>) {
    if let Some(agent) = acp_agent {
        options.insert("acp_test".to_string(), Value::Bool(true));
        options.insert("acp_agent".to_string(), Value::String(agent.to_string()));
    }
}

/// 组装仅供运行时使用的标题请求 profile。
///
/// 标题生成不是用户可选 Agent：它没有配置项、系统提示词或工具权限，只有一次纯文本请求。
fn internal_title_profile(acp_agent: Option<&str>) -> crate::app::agent::session::llm::AgentInfo {
    let mut options = HashMap::new();
    apply_acp_agent_options(&mut options, acp_agent);

    if acp_agent.is_some() {
        options.extend([
            ("full_access".into(), Value::Bool(false)),
            ("acp_permission_mode".into(), serde_json::json!("deny-all")),
            ("acp_non_interactive_permissions".into(), serde_json::json!("deny")),
            ("acp_force_new_session".into(), Value::Bool(true)),
            ("acp_close_client_after_request".into(), Value::Bool(true)),
        ]);
    }

    crate::app::agent::session::llm::AgentInfo {
        name: "session-title".to_string(),
        mode: INTERNAL_TITLE_MODE.to_string(),
        prompt: None,
        temperature: None,
        top_p: None,
        options,
        permission: Default::default(),
    }
}

async fn stream_title(
    session_id: String,
    mut user: crate::app::agent::session::message::UserInfo,
    model: provider::Model,
    prompt: String,
    acp_agent: Option<&str>,
) -> Result<String, crate::app::agent::session::llm::Error> {
    use std::sync::{Arc, Mutex};

    let output = Arc::new(Mutex::new(String::new()));
    let output_clone = Arc::clone(&output);
    let mut profile = internal_title_profile(acp_agent);

    tracing::info!(
        target: "vw_agent",
        session_id = %session_id,
        operation = "session_title",
        provider_id = %model.provider_id,
        model_id = %model.id,
        acp_test = acp_agent.is_some(),
        acp_agent = ?acp_agent,
        "starting internal session title generation"
    );

    if acp_agent.is_some() {
        let directory = crate::project::instance::directory();
        profile.options.insert("cwd".into(), Value::String(directory));
    }
    // Keep auxiliary model usage and ACP state separate from the conversation.
    let session_id = format!("session-title:{}", uuid::Uuid::new_v4());
    user.session_id.clone_from(&session_id);

    Box::pin(crate::app::agent::session::llm::stream(
        crate::app::agent::session::llm::StreamInput {
            agent: profile,
            user,
            session_id,
            model,
            system: Vec::new(),
            abort: None,
            messages: vec![serde_json::json!({ "role": "user", "content": prompt })],
            small: true,
            tools: Default::default(),
            retries: 1,
            retry_in_caller: false,
        },
        move |event| {
            if let crate::app::agent::session::llm::StreamEvent::Delta(d) = event
                && let Ok(mut output) = output_clone.lock()
            {
                output.push_str(&d);
            }
        },
    ))
    .await?;

    Ok(output.lock().unwrap_or_else(|e| e.into_inner()).clone())
}

fn finalize_generated_title(output: &str) -> Result<String, String> {
    let title = output.trim().to_string();
    if title.is_empty() {
        return Err("Empty title generated".to_string());
    }

    if title.chars().count() > MAX_TITLE_CHARS {
        Ok(title.chars().take(TITLE_TRUNCATE_KEEP).collect::<String>() + "...")
    } else {
        Ok(title)
    }
}

/// 根据会话内容生成标题
///
/// 该函数接收会话 ID 和内容，通过调用 LLM 生成简洁的会话标题。
/// 支持指定模型；未指定模型或选择 auto 时复用运行时的自动选择策略。
///
/// # 参数
///
/// - `session_id`: 会话唯一标识符，用于关联 LLM 调用上下文
/// - `content`: 用于生成标题的原始内容（通常是会话的首条消息）
/// - `preferred_model`: 可选的优先模型标识，格式支持：
///   - 完整格式：`"provider/model"`（如 `"anthropic/claude-3-sonnet"`）
///   - 简短格式：`"model"`（如 `"gpt-4"`，自动在各 provider 中查找）
///
/// # 返回值
///
/// - `Ok(String)`: 生成的标题，长度不超过 50 字符
/// - `Err(String)`: 错误信息，可能原因包括：
///   - 输入内容为空
///   - 未找到可用的模型
///   - LLM 调用失败
///   - 生成的标题为空
///
/// # 示例
///
/// ```rust,ignore
/// // 使用默认模型生成标题
/// let title = generate_from_content(
///     "sess-001".to_string(),
///     "你好，请问如何配置环境变量？".to_string(),
///     None,
/// ).await?;
///
/// // 指定优先模型
/// let title = generate_from_content(
///     "sess-002".to_string(),
///     "帮我分析这段代码的性能问题".to_string(),
///     Some("openai/gpt-4".to_string()),
/// ).await?;
/// ```
///
/// # 实现细节
///
/// 1. **内容预处理**：截断过长内容（超过 200 字符），避免 LLM token 浪费
/// 2. **模型解析与回退**：
///    - 显式模型必须可用，短名称必须唯一
///    - 未指定或选择 auto 时，从 active 模型中自动选择
/// 3. **LLM 调用**：构造提示词并流式调用 LLM
/// 4. **后处理**：截断过长标题（超过 50 字符），添加省略号
pub async fn generate_from_content(
    session_id: String,
    content: String,
    preferred_model: Option<String>,
    acp_agent: Option<String>,
) -> Result<String, String> {
    // 内容预处理：截断过长内容以节省 token 消耗
    let truncated = truncate_title_source(content);

    // 空内容检查：避免无意义的 LLM 调用
    if truncated.trim().is_empty() {
        return Err("Empty title source".to_string());
    }

    let requested_model = preferred_model
        .as_deref()
        .map(str::trim)
        .filter(|model| !model.is_empty() && !model.eq_ignore_ascii_case("auto"));
    let model = match requested_model {
        None => {
            crate::session::llm::resolve_auto_model(
                &build_title_prompt(&truncated),
                &Value::Null,
                false,
            )
            .await?
        }
        Some(requested) => {
            let parsed = provider::parse_model(requested);
            if requested.contains('/') {
                provider::get_model(&parsed.provider_id, &parsed.model_id)
                    .await
                    .map_err(|error| error.to_string())?
            } else {
                let providers = provider::list().await.map_err(|error| error.to_string())?;
                let mut matches = providers.values().filter_map(|info| info.models.get(requested));
                let model = matches
                    .next()
                    .cloned()
                    .ok_or_else(|| format!("Title model not found: {requested}"))?;
                if matches.next().is_some() {
                    return Err(format!("Ambiguous title model: {requested}; use provider/model"));
                }
                model
            }
        }
    };

    let output = stream_title(
        session_id.clone(),
        crate::app::agent::session::message::UserInfo {
            id: "title_gen".to_string(),
            session_id,
            time: crate::app::agent::session::message::UserTime {
                created: crate::app::agent::session::session::now_ms(),
            },
            summary: None,
            agent: "session-title".to_string(),
            model: crate::app::agent::session::message::ModelRef {
                provider_id: model.provider_id.clone(),
                model_id: model.id.clone(),
            },
            system: None,
            tools: None,
            variant: None,
        },
        model,
        build_title_prompt(&truncated),
        acp_agent.as_deref(),
    )
    .await
    .map_err(|e| format!("LLM stream error: {e}"))?;

    finalize_generated_title(&output)
}

/// 为会话中已持久化的用户消息生成标题。
///
/// 此路径优先使用同一提供商的小型模型，再回退到该消息实际使用的模型，避免改变
/// 会话元数据的成本与路由语义。
pub(crate) async fn generate_message_title(
    session_id: &str,
    user: crate::app::agent::session::message::UserInfo,
    content: String,
) -> Result<Option<String>, crate::app::agent::session::llm::Error> {
    let content = truncate_title_source(content);
    if content.trim().is_empty() {
        return Ok(None);
    }

    let model = match provider::get_small_model(&user.model.provider_id).await.map_err(|error| {
        crate::session::llm::Error::Api(crate::session::message::AssistantError::Unknown {
            message: error.to_string(),
        })
    })? {
        Some(model) => Some(model),
        None => match provider::get_model(&user.model.provider_id, &user.model.model_id).await {
            Ok(model) => Some(model),
            Err(provider::CatalogError::ModelNotFound(_)) => None,
            Err(error) => {
                return Err(crate::session::llm::Error::Api(
                    crate::session::message::AssistantError::Unknown { message: error.to_string() },
                ));
            }
        },
    };
    let Some(model) = model else { return Ok(None) };

    let output =
        stream_title(session_id.to_string(), user, model, build_title_prompt(&content), None)
            .await?;
    let title = output.trim();
    Ok((!title.is_empty()).then(|| title.to_string()))
}
#[cfg(test)]
#[path = "title_tests.rs"]
mod title_tests;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn spawn_first_response_title(
    runtime: &tokio::runtime::Handle,
    tenant: vw_api_types::id::TenantScope,
    context: crate::project::instance::Context,
    session_id: String,
    preview: String,
    preferred_model: Option<String>,
) {
    runtime.spawn(crate::session::ui_store::with_tenant_scope(tenant.clone(), async move {
        let project_id = context.project.id.clone();
        let generate = crate::project::instance::provide_context(
            context,
            Box::pin(generate_from_content(session_id.clone(), preview, preferred_model, None)),
        );
        // The native, tool-free title prompt also serves conversations run through ACP.
        if let Ok(Ok(title)) =
            tokio::time::timeout(std::time::Duration::from_secs(30), generate).await
        {
            let result = crate::global_database::run_isolated(move || {
                crate::session::ui_store::set_first_response_title(
                    &tenant,
                    &project_id,
                    &session_id,
                    &title,
                )
            })
            .await;
            if let Err(error) = result.and_then(|result| result) {
                tracing::warn!(%error, "failed to persist first response title");
            }
        }
    }));
}
