//! 会话标题生成模块
//!
//! 本模块提供基于会话内容自动生成标题的功能。通过调用大语言模型（LLM），
//! 根据用户的对话内容生成简洁、准确的会话标题，便于会话管理和检索。
//!
//! # 主要功能
//!
//! - 基于会话首条消息内容生成标题
//! - 支持优先使用指定模型，回退到系统默认模型
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

fn truncate_title_source(content: String) -> String {
    if content.chars().count() > CONTENT_TRUNCATE_CHARS {
        content.chars().take(CONTENT_TRUNCATE_CHARS).collect::<String>()
    } else {
        content
    }
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
/// 支持指定优先使用的模型，若不可用则自动回退到系统配置的默认模型。
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
///   - 未找到可用的标题生成代理配置
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
///    - 尝试解析并获取指定的优先模型
///    - 若无指定或不可用，回退到代理配置的默认模型
///    - 若仍不可用，依次尝试 Anthropic 和 OpenAI 的小型模型
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

    // 获取标题生成代理的配置信息
    let agent_info = crate::app::agent::agent::get("title").await;
    let Some(agent_info) = agent_info else {
        return Err("No title agent found".to_string());
    };

    // 解析优先模型：支持 "provider/model" 和 "model" 两种格式
    let preferred_resolved = if let Some(s) = preferred_model.as_ref() {
        if s.contains('/') {
            // 完整格式：直接解析 provider/model
            let parsed = provider::parse_model(s);
            provider::get_model(&parsed.provider_id, &parsed.model_id).await.ok()
        } else {
            // 简短格式：遍历所有 provider 查找匹配的模型
            let providers = provider::list().await;
            let mut hit: Option<provider::Model> = None;
            for (provider_id, info) in providers {
                if info.models.contains_key(s)
                    && let Ok(model) = provider::get_model(&provider_id, s).await
                {
                    hit = Some(model);
                    break;
                }
            }
            hit
        }
    } else {
        None
    };

    // 确定最终使用的模型（按优先级回退）
    let model = if preferred_resolved.is_some() {
        // 优先级 1：用户指定的模型
        preferred_resolved
    } else if let Some(m) = crate::app::agent::agent::resolve_model_ref(&agent_info).as_ref() {
        // 优先级 2：代理配置的默认模型
        provider::get_model(&m.provider_id, &m.model_id).await.ok()
    } else {
        // 优先级 3：系统小型模型（先尝试 Anthropic，再尝试 OpenAI）
        match provider::get_small_model("anthropic").await {
            Some(m) => Some(m),
            None => provider::get_small_model("openai").await,
        }
    };

    // 确保有可用模型
    let Some(model) = model else {
        return Err("No model available for title generation".to_string());
    };

    // 构造标题生成提示词
    let prompt = build_title_prompt(&truncated);

    // 准备流式输出收集器
    use std::sync::{Arc, Mutex};
    let output = Arc::new(Mutex::new(String::new()));
    let output_clone = Arc::clone(&output);

    // 构造 LLM 代理信息
    let mut options = agent_info.options.clone();
    apply_acp_agent_options(&mut options, acp_agent.as_deref());
    let llm_agent = crate::app::agent::session::llm::AgentInfo {
        name: "title".to_string(),
        mode: agent_info.mode.clone(),
        prompt: agent_info.system_prompt.clone(),
        temperature: agent_info.temperature,
        top_p: agent_info.top_p,
        options,
        permission: crate::app::agent::agent::permission_rules("title").await.unwrap_or_default(),
    };
    tracing::info!(
        target: "vw_agent",
        session_id = %session_id,
        agent = "title",
        provider_id = %model.provider_id,
        model_id = %model.id,
        acp_test = acp_agent.is_some(),
        acp_agent = ?acp_agent,
        "starting session title generation"
    );

    // 调用 LLM 流式接口生成标题
    crate::app::agent::session::llm::stream(
        crate::app::agent::session::llm::StreamInput {
            agent: llm_agent,
            user: crate::app::agent::session::message::UserInfo {
                id: "title_gen".to_string(),
                session_id: session_id.clone(),
                time: crate::app::agent::session::message::UserTime {
                    created: crate::app::agent::session::session::now_ms(),
                },
                summary: None,
                agent: "title".to_string(),
                model: crate::app::agent::session::message::ModelRef {
                    provider_id: model.provider_id.clone(),
                    model_id: model.id.clone(),
                },
                system: None,
                tools: None,
                variant: None,
            },
            session_id: session_id.clone(),
            model,
            system: Vec::new(),
            abort: None,
            messages: vec![serde_json::json!({ "role": "user", "content": prompt })],
            small: true,
            tools: Default::default(),
            retries: 1,
        },
        // 流式事件回调：收集 LLM 输出的文本片段
        move |event| {
            if let crate::app::agent::session::llm::StreamEvent::Delta(d) = event
                && let Ok(mut s) = output_clone.lock()
            {
                s.push_str(&d);
            }
        },
    )
    .await
    .map_err(|e| format!("LLM stream error: {}", e))?;

    // 提取并清理生成的标题
    let output = output.lock().unwrap_or_else(|e| e.into_inner());
    finalize_generated_title(&output)
}
#[cfg(test)]
#[path = "title_tests.rs"]
mod title_tests;
