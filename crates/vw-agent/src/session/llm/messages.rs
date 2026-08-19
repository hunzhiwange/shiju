//! LLM 消息构建与处理模块
//!
//! 本模块负责构建和管理发送给大语言模型（LLM）的消息结构。
//! 主要功能包括：
//! - 构建系统消息（system messages）
//! - 构建聊天消息序列（chat messages）
//! - 确保 OpenAI 工具调用序列的合法性
//! - 解析和过滤可用工具列表
//! - 检测消息中是否包含工具调用
//!
//! # 模块结构
//!
//! - [`build_system_messages`] - 构建系统提示词消息
//! - [`build_chat_messages`] - 构建完整的聊天消息序列
//! - [`resolve_tools`] - 根据权限和用户配置过滤工具
//! - [`has_tool_calls`] - 检测消息中是否存在工具调用

use crate::app::agent::provider::provider;
use crate::app::agent::provider::transform as provider_transform;
use serde_json::{Value, json};

use super::logging::LOGGER;
use super::types::StreamInput;

/// 构建系统消息列表
///
/// 根据输入参数组装完整的系统提示词消息，按以下顺序合并：
/// 1. Agent 的基础提示词（prompt）
/// 2. 统一会话系统提示（`system::provider`）
/// 3. 输入中的系统消息
/// 4. 用户自定义的系统消息
///
/// Codex OAuth 路径不再二次注入系统提示正文：`provider` 与
/// `instructions` 已是同一正文，重复追加会导致整文翻倍。`is_codex`
/// 仍保留给调用方做选项/token 等处理，本函数对提示正文无额外操作。
///
/// # 参数
///
/// * `input` - 流输入数据，包含 agent 配置、模型信息和系统消息
/// * `is_codex` - 是否为 Codex 模式（仅用于日志；不二次追加统一提示）
///
/// # 返回值
///
/// 返回一个字符串向量，包含合并后的系统消息。
/// 如果所有部分都为空，则返回空向量。
///
/// # 示例
///
/// ```ignore
/// let input = StreamInput {
///     agent: AgentInfo { prompt: Some("你是助手".to_string()), ... },
///     model: ModelInfo { ... },
///     system: vec!["系统规则".to_string()],
///     user: UserInfo { system: Some("用户规则".to_string()), ... },
///     ...
/// };
/// let system_msgs = build_system_messages(&input, false);
/// // system_msgs 将包含所有合并后的系统消息
/// ```
pub fn build_system_messages(input: &StreamInput, is_codex: bool) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();

    // 添加 Agent 的基础提示词（如果存在）
    if let Some(p) = input.agent.prompt.as_ref() {
        parts.push(p.clone());
    }

    // 记录日志：开始处理 provider 提示词
    LOGGER
        .clone_logger()
        .tag("providerID", &input.model.provider_id)
        .tag("modelID", &input.model.api.id)
        .tag("is_codex", &is_codex.to_string())
        .info("build_system_messages: before provider prompt", None);

    // 主会话会在知道真实工具与技能后预先渲染统一提示；旧入口则在这里
    // 根据 StreamInput 中的工具规格生成安全降级版本。
    let unified_already_rendered = input
        .system
        .iter()
        .any(|part| part.contains(crate::app::agent::session::system::UNIFIED_PROMPT_MARKER));
    let provider_prompts = if unified_already_rendered {
        Vec::new()
    } else {
        let tool_ids = input.tools.keys().cloned().collect::<Vec<_>>();
        crate::app::agent::session::system::provider_with_context(
            &input.model,
            None,
            &tool_ids,
            false,
        )
    };

    // 记录日志：provider 提示词数量
    LOGGER
        .clone_logger()
        .tag("providerID", &input.model.provider_id)
        .tag("modelID", &input.model.api.id)
        .tag("count", &provider_prompts.len().to_string())
        .info("build_system_messages: provider prompts", None);

    // 将 provider 提示词添加到消息列表
    parts.extend(provider_prompts.into_iter().map(|s| s.to_string()));

    // 添加输入中的系统消息（过滤掉空白消息）
    parts.extend(input.system.iter().cloned().filter(|s| !s.trim().is_empty()));

    // 添加用户自定义的系统消息（如果非空）
    if let Some(s) = input.user.system.as_ref() {
        if !s.trim().is_empty() {
            parts.push(s.clone());
        }
    }

    // 合并所有非空部分，用换行符连接
    let joined = parts.into_iter().filter(|s| !s.trim().is_empty()).collect::<Vec<_>>().join("\n");

    // 记录日志：最终合并结果的长度
    LOGGER
        .clone_logger()
        .tag("providerID", &input.model.provider_id)
        .tag("modelID", &input.model.api.id)
        .tag("joined_len", &joined.len().to_string())
        .info("build_system_messages: final joined", None);

    // 如果合并结果为空则返回空向量，否则返回包含合并后消息的向量
    if joined.is_empty() { Vec::new() } else { vec![joined] }
}

/// 构建聊天消息序列
///
/// 将系统消息和用户消息合并为完整的聊天消息序列，
/// 并进行必要的转换和规范化处理。
///
/// # 参数
///
/// * `system` - 系统消息列表
/// * `msgs` - 用户消息列表（JSON 格式）
/// * `model` - 模型配置信息
///
/// # 返回值
///
/// 返回一个 JSON 数组，包含所有消息的规范化表示。
///
/// # 处理流程
///
/// 1. 将系统消息转换为标准格式
/// 2. 合并用户消息
/// 3. 根据模型适配器规范化消息格式
/// 4. 应用 provider 特定的键名重映射
/// 5. 确保 OpenAI 工具调用序列的合法性
/// 6. 根据 provider 配置决定是否保留推理内容（reasoning_content）
///
/// # 示例
///
/// ```ignore
/// let system = vec!["你是一个助手".to_string()];
/// let msgs = vec![json!({ "role": "user", "content": "你好" })];
/// let model = get_model_info();
/// let chat_msgs = build_chat_messages(&system, &msgs, &model);
/// ```
pub fn build_chat_messages(system: &[String], msgs: &[Value], model: &provider::Model) -> Value {
    let mut out: Vec<Value> = Vec::new();

    // 将每条系统消息转换为标准 JSON 格式
    for s in system {
        out.push(json!({ "role": "system", "content": s }));
    }

    // 添加用户消息
    out.extend(msgs.iter().cloned());

    // 根据模型适配器规范化消息格式
    let out = provider_transform::normalize_messages(out, &model.api.adapter, &model.api.id);

    // 应用 provider 特定的键名重映射
    let mut out = provider_transform::apply_provider_options_key_remap(
        out,
        &model.provider_id,
        &model.api.adapter,
    );

    // 确保工具调用序列符合 OpenAI 规范
    ensure_openai_tool_call_sequence(&mut out);

    // 检查是否允许保留推理内容（仅 DeepSeek 相关模型支持）
    let allow_reasoning_content = model.api.id.contains("deepseek-reasoner")
        || model.provider_id.to_ascii_lowercase().contains("deepseek");

    // 如果不允许推理内容，从所有消息中移除 reasoning_content 字段
    if !allow_reasoning_content {
        for msg in &mut out {
            if let Some(obj) = msg.as_object_mut() {
                obj.remove("reasoning_content");
            }
        }
    }

    Value::Array(out)
}

/// 确保 OpenAI 工具调用序列的合法性
///
/// 根据 OpenAI API 的要求，当 assistant 消息包含 tool_calls 时，
/// 必须紧随对应的 tool 消息作为响应。此函数会检查并自动补齐
/// 缺失的 tool 响应消息。
///
/// # 参数
///
/// * `msgs` - 消息列表的可变引用
///
/// # 处理逻辑
///
/// 1. 遍历所有 assistant 消息，提取其中的 tool_calls ID
/// 2. 检查紧随其后是否有对应的 tool 响应消息
/// 3. 如果缺失，自动插入占位的 tool 响应消息
/// 4. 确保每个 tool_call 都有对应的 tool 响应
///
/// # 注意
///
/// 补齐的 tool 响应消息会包含特殊标记，提示不要重复执行命令。
fn ensure_openai_tool_call_sequence(msgs: &mut Vec<Value>) {
    let mut i = 0usize;

    // 遍历所有消息
    while i < msgs.len() {
        // 提取当前消息的角色、tool_calls ID 和是否有 content
        let (role, ids, has_content) = {
            let Some(obj) = msgs[i].as_object() else {
                i += 1;
                continue;
            };

            let role = obj.get("role").and_then(Value::as_str).unwrap_or_default().to_string();

            // 只处理 assistant 消息
            if role != "assistant" {
                (role, Vec::new(), true)
            } else {
                // 提取 tool_calls 中的所有 ID
                let ids = obj
                    .get("tool_calls")
                    .and_then(Value::as_array)
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|tc| tc.get("id").and_then(Value::as_str))
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                (role, ids, obj.contains_key("content"))
            }
        };

        // 如果不是 assistant 消息或没有 tool_calls，跳过
        if role != "assistant" || ids.is_empty() {
            i += 1;
            continue;
        }

        // 如果 assistant 消息没有 content，添加一个空格作为占位符
        // OpenAI API 要求 assistant 消息必须有 content 字段
        if !has_content {
            if let Some(obj) = msgs[i].as_object_mut() {
                obj.insert("content".to_string(), Value::String(" ".to_string()));
            }
        }

        // 收集紧随其后的 tool 响应消息中已出现的 tool_call_id
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut j = i + 1;

        while j < msgs.len() {
            let Some(next_obj) = msgs[j].as_object() else { break };
            let next_role = next_obj.get("role").and_then(Value::as_str).unwrap_or_default();

            // 只处理连续的 tool 消息
            if next_role != "tool" {
                break;
            }

            // 如果 tool 消息的 tool_call_id 在当前 assistant 的 tool_calls 中，记录为已见到
            if let Some(tool_call_id) = next_obj.get("tool_call_id").and_then(Value::as_str) {
                if ids.iter().any(|id| id == tool_call_id) {
                    seen.insert(tool_call_id.to_string());
                }
            }
            j += 1;
        }

        // 检查是否有缺失的 tool 响应
        if seen.len() != ids.len() {
            // 收集所有缺失的 tool_call_id
            let mut missing = Vec::new();
            for id in &ids {
                if !seen.contains(id) {
                    missing.push(id.clone());
                }
            }

            // 为每个缺失的 tool_call_id 插入占位的 tool 响应消息
            for (k, id) in missing.into_iter().enumerate() {
                let content = format!(
                    "<terminal_metadata>\nmissing_tool_result: true\ntool_call_id: {id}\n原因: 历史消息中存在 tool_calls 但缺少对应 tool 结果；为保持工具调用序列合法而自动补齐。\n建议: 不要重复执行同一命令；请以终端实际输出为准，或重新执行并确保工具结果被记录。\n</terminal_metadata>"
                );
                msgs.insert(
                    j + k,
                    json!({ "role": "tool", "tool_call_id": id, "content": content }),
                );
            }
        }
        i += 1;
    }
}

/// 解析并过滤可用工具列表
///
/// 根据用户权限配置和用户偏好过滤工具列表，
/// 确保只有被允许的工具可用。
///
/// # 参数
///
/// * `tools` - 所有工具的映射表（工具名 -> 工具规格）
/// * `permission` - 权限规则集
/// * `user` - 用户信息，包含用户的工具偏好设置
///
/// # 返回值
///
/// 返回过滤后的工具映射表，只包含被允许使用的工具。
///
/// # 过滤规则
///
/// 1. 如果用户明确禁用了某个工具（设置为 false），则移除
/// 2. 如果权限规则禁用了某个工具，则移除
/// 3. 其他工具保留
///
/// # 示例
///
/// ```ignore
/// let tools = get_all_tools();
/// let permission = get_permission_ruleset();
/// let user = get_user_info();
/// let available_tools = resolve_tools(&tools, &permission, &user);
/// ```
pub fn resolve_tools(
    tools: &std::collections::HashMap<String, crate::app::agent::tools::ToolSpec>,
    permission: &crate::app::agent::permission::next::Ruleset,
    user: &crate::app::agent::session::message::UserInfo,
) -> std::collections::HashMap<String, crate::app::agent::tools::ToolSpec> {
    // 克隆工具列表作为输出
    let mut out = tools.clone();

    // 获取所有工具名称
    let tool_names = out.keys().cloned().collect::<Vec<_>>();

    // 根据权限规则获取被禁用的工具列表
    let disabled = crate::app::agent::permission::next::disabled(&tool_names, permission);

    // 遍历所有工具，根据用户偏好和权限规则进行过滤
    for name in tool_names {
        // 如果用户明确禁用了该工具，移除
        if user.tools.as_ref().and_then(|m| m.get(&name)).copied() == Some(false) {
            out.remove(&name);
            continue;
        }
        // 如果权限规则禁用了该工具，移除
        if disabled.contains(&name) {
            out.remove(&name);
        }
    }

    out
}

/// 检测消息中是否包含工具调用
///
/// 遍历消息列表，检查是否存在任何形式的工具调用或工具响应。
///
/// # 参数
///
/// * `messages` - 消息列表（JSON 格式）
///
/// # 返回值
///
/// 如果消息中包含工具调用或工具响应，返回 `true`；否则返回 `false`。
///
/// # 检测条件
///
/// 满足以下任一条件即返回 `true`：
/// 1. 消息中存在非空的 `tool_calls` 数组
/// 2. 消息角色为 `tool` 且包含 `tool_call_id`
/// 3. 消息内容为数组，且包含 `tool-call` 或 `tool-result` 类型的部分
///
/// # 示例
///
/// ```ignore
/// let messages = vec![
///     json!({ "role": "assistant", "tool_calls": vec![{"id": "call_123"}] })
/// ];
/// assert!(has_tool_calls(&messages));
///
/// let messages = vec![
///     json!({ "role": "user", "content": "你好" })
/// ];
/// assert!(!has_tool_calls(&messages));
/// ```
pub fn has_tool_calls(messages: &[Value]) -> bool {
    for msg in messages {
        // 检查是否存在 tool_calls 数组且非空
        if msg.get("tool_calls").and_then(Value::as_array).is_some_and(|a| !a.is_empty()) {
            return true;
        }

        // 检查是否为 tool 响应消息（角色为 tool 且包含 tool_call_id）
        if msg.get("role").and_then(Value::as_str) == Some("tool")
            && msg.get("tool_call_id").and_then(Value::as_str).is_some()
        {
            return true;
        }

        // 检查消息内容是否为数组格式，并查找工具调用相关的类型
        let Some(content) = msg.get("content") else { continue };
        let Some(arr) = content.as_array() else { continue };

        for part in arr {
            let Some(ty) = part.get("type").and_then(Value::as_str) else { continue };
            // 检查是否为 tool-call 或 tool-result 类型
            if ty == "tool-call" || ty == "tool-result" {
                return true;
            }
        }
    }
    false
}
#[cfg(test)]
#[path = "messages_tests.rs"]
mod messages_tests;
