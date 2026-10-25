//! 待办事项更新处理器模块
//!
//! 本模块负责在 Agent 执行工作后自动更新待办事项列表的状态。
//! 当启用 `todowrite` 工具且存在未完成的待办事项时，会触发 LLM
//! 分析执行进展并更新 todo 状态。
//!
//! # 主要功能
//!
//! - 检查是否需要更新待办事项（基于工具可用性和待办状态）
//! - 调用 LLM 分析执行进展
//! - 执行 `todowrite` 工具更新待办事项状态
//! - 记录 token 使用情况

use super::types::StreamEvent;
use crate::app::agent::tools::{TODO_WRITE_TOOL_ALIAS, TODO_WRITE_TOOL_ID, is_todo_write_tool_id};
use crate::session::ui_types as models;
use std::collections::HashSet;

/// 在工作执行后可能更新待办事项列表
///
/// 此函数检查是否需要根据执行进展更新待办事项状态。
/// 只有在以下条件全部满足时才会触发更新：
/// 1. `todowrite` 工具在允许的工具列表中
/// 2. 存在未完成的待办事项
/// 3. 待办事项列表不为空
///
/// # 参数
///
/// * `session` - 会话实例的可变引用，用于执行工具和记录状态
/// * `ctx` - 工具执行上下文，提供待办事项的读写能力
/// * `model` - 可选的 LLM 模型名称，用于指定调用哪个模型
/// * `base_system` - 基础系统提示词，会追加 todo 更新指令
/// * `allowed_tools` - 允许使用的工具集合，用于检查 `todowrite` 是否可用
/// * `llm_messages` - LLM 消息历史，可变引用用于追加助手消息和工具结果
/// * `total_usage` - 累计 token 使用量，可变引用用于累加本次消耗
/// * `on_event` - 流事件回调函数，用于处理执行过程中的事件
/// * `tool_state` - 工具会话状态，可变引用用于记录工具执行状态
///
/// # 返回值
///
/// 返回 `bool` 值：
/// - `true`：成功执行了至少一次 `todowrite` 工具调用
/// - `false`：未满足更新条件或执行失败
///
/// # 示例
///
/// ```ignore
/// let updated = maybe_update_todos_after_work(
///     &mut session,
///     &ctx,
///     Some("gpt-4".to_string()),
///     &base_system,
///     &allowed_tools,
///     &mut llm_messages,
///     &mut total_usage,
///     &mut |event| true,
///     &mut tool_state,
/// );
/// if updated {
///     println!("待办事项已更新");
/// }
/// ```
pub(crate) fn maybe_update_todos_after_work(
    session: &mut super::Session,
    ctx: &crate::app::agent::tools::ToolRuntimeContext,
    model: Option<String>,
    base_system: &str,
    allowed_tools: &HashSet<String>,
    llm_messages: &mut Vec<serde_json::Value>,
    total_usage: &mut models::TokenUsage,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    tool_state: &mut super::ToolSessionState,
    reliability: &crate::app::agent::config::ReliabilityConfig,
) -> bool {
    // 检查 todowrite 工具是否在允许列表中，若不存在则直接返回
    if !allowed_tools.iter().any(|name| is_todo_write_tool_id(name)) {
        return false;
    }
    // 检查是否存在未完成的待办事项，若没有则无需更新
    if !super::todos::has_incomplete_todos(ctx) {
        return false;
    }
    // 检查待办事项列表是否为空，为空则跳过更新
    if super::todos::read_todos_or_empty(ctx).is_empty() {
        return false;
    }

    // 构造仅包含 todowrite 工具的集合，限制 LLM 只能调用此工具
    let mut only_todowrite = HashSet::<String>::new();
    only_todowrite.insert(TODO_WRITE_TOOL_ID.to_string());
    only_todowrite.insert(TODO_WRITE_TOOL_ALIAS.to_string());

    // 空的事件处理器（LLM 调用阶段不需要处理事件）
    let mut noop = |_ev: StreamEvent| true;
    let available_tool_specs = crate::app::agent::tools::tool_specs_for_context(ctx);

    // 构造系统提示词：基础提示 + todo 更新指令
    let system = [
        base_system.to_string(),
        "请根据刚刚的执行进展，仅通过 TodoWrite 更新 todo 状态；不要输出普通文本。".to_string(),
    ];

    // 调用 LLM 分析执行进展并生成工具调用，最多重试 2 次
    let step_out = match super::llm_runner::run_llm_step_with_retry(
        &ctx.session,
        &llm_messages,
        &system,
        model,
        &serde_json::Value::Object(serde_json::Map::new()),
        &available_tool_specs,
        &only_todowrite,
        &mut noop,
        reliability.provider_retries,
        reliability.provider_backoff_ms,
        ctx.tool_use_context().abort_token(),
    ) {
        Ok(v) => v,
        Err(_) => return false, // LLM 调用失败，返回 false
    };

    apply_todo_update_step(llm_messages, total_usage, step_out, |call| {
        super::tools_exec::run_tool_and_record(
            session,
            &call.name,
            &call.arguments,
            ctx,
            true,
            on_event,
            tool_state,
        )
        .unwrap_or_default()
    })
}

fn apply_todo_update_step(
    llm_messages: &mut Vec<serde_json::Value>,
    total_usage: &mut models::TokenUsage,
    step_out: super::llm_runner::LlmStep,
    mut run_todowrite: impl FnMut(&crate::app::agent::session::llm::ToolCall) -> String,
) -> bool {
    // 累加本次 LLM 调用的 token 使用量到总计中
    total_usage.input_tokens += step_out.usage.input_tokens;
    total_usage.output_tokens += step_out.usage.output_tokens;
    total_usage.cached_tokens += step_out.usage.cached_tokens;
    total_usage.reasoning_tokens += step_out.usage.reasoning_tokens;

    // 如果 LLM 未返回任何工具调用，直接返回 false
    if step_out.tool_calls.is_empty() {
        return false;
    }

    // 将助手消息（包含工具调用和推理内容）追加到消息历史
    llm_messages.push(super::utils::tool_calls_to_assistant_message(
        &step_out.tool_calls,
        &step_out.reasoning_content,
    ));

    // 标记是否实际执行了 todowrite 工具
    let mut ran = false;

    // 遍历所有工具调用，执行 todowrite 工具
    for call in step_out.tool_calls {
        // 跳过非 todowrite 工具调用（理论上不会有其他工具）
        if !is_todo_write_tool_id(&call.name) {
            continue;
        }
        ran = true; // 标记已执行

        // 执行工具并记录结果；调用方负责将执行失败转换为空字符串
        let content = run_todowrite(&call);

        // 将工具执行结果追加到消息历史
        llm_messages.push(super::utils::tool_result_to_message(&call.id, &content));
    }

    // 返回是否实际执行了 todowrite 工具
    ran
}
#[cfg(test)]
#[path = "todo_updates_tests.rs"]
mod todo_updates_tests;
