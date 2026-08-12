//! 任务列表（Todo）辅助处理模块
//!
//! 本模块提供会话处理过程中的任务列表状态检查与更新功能。
//! 主要用于：
//! - 检查任务完成状态
//! - 读取任务列表数据
//! - 批量更新任务状态
//!
//! 这些函数支持会话处理器在适当时机自动更新任务状态，
//! 例如标记所有任务为已完成或将第一个任务标记为进行中。

use super::types::StreamEvent;
use crate::app::agent::tools::ToolRuntimeContext;
use crate::app::agent::tools::todo;

/// 检查是否存在未完成的任务
///
/// 读取任务列表并检查是否有状态不为 "completed" 的任务。
///
/// # 参数
///
/// - `ctx`: 工具执行上下文，提供访问任务列表所需的配置和路径信息
///
/// # 返回值
///
/// 返回 `true` 表示存在至少一个未完成的任务；
/// 返回 `false` 表示所有任务都已完成，或读取失败，或解析失败。
///
/// # 示例
///
/// ```ignore
/// if has_incomplete_todos(&ctx) {
///     println!("还有未完成的任务");
/// }
/// ```
pub(crate) fn has_incomplete_todos(ctx: &ToolRuntimeContext) -> bool {
    // 尝试读取原始任务列表数据，失败则返回 false
    let Ok(raw) = todo::read(ctx) else {
        return false;
    };
    // 尝试解析为任务列表，失败则返回 false
    let Ok(items) = serde_json::from_str::<Vec<todo::Todo>>(raw.trim()) else {
        return false;
    };
    // 检查是否有任务状态不是 "completed"
    items.iter().any(|t| t.status != "completed")
}

/// 读取任务列表，失败时返回空列表
///
/// 尝试从存储中读取并解析任务列表。如果读取或解析失败，返回空列表而不是错误。
///
/// # 参数
///
/// - `ctx`: 工具执行上下文，提供访问任务列表所需的配置和路径信息
///
/// # 返回值
///
/// 成功时返回任务列表的克隆；
/// 读取或解析失败时返回空的 `Vec<todo::Todo>`。
///
/// # 示例
///
/// ```ignore
/// let todos = read_todos_or_empty(&ctx);
/// println!("当前任务数量: {}", todos.len());
/// ```
pub(crate) fn read_todos_or_empty(ctx: &ToolRuntimeContext) -> Vec<todo::Todo> {
    // 尝试读取原始任务列表数据，失败则返回空列表
    let Ok(raw) = todo::read(ctx) else {
        return Vec::new();
    };
    // 解析任务列表，失败时返回默认值（空列表）
    serde_json::from_str::<Vec<todo::Todo>>(raw.trim()).unwrap_or_default()
}

/// 构建任务状态更新补丁列表
///
/// 根据任务列表生成需要更新的状态补丁。可以更新所有符合条件的任务，
/// 或仅更新第一个符合条件的任务。
///
/// # 参数
///
/// - `items`: 任务列表引用
/// - `target_status`: 目标状态（如 "completed"、"in_progress" 等）
/// - `only_first_incomplete`: 是否仅处理第一个未完成的任务
///   - `true`: 只更新第一个需要改变状态的任务
///   - `false`: 更新所有需要改变状态的任务
///
/// # 返回值
///
/// 返回包含状态更新信息的 JSON 对象列表，每个对象包含：
/// - `id`: 任务 ID
/// - `status`: 目标状态
///
/// # 处理逻辑
///
/// - 跳过已完成的任务（status == "completed"）
/// - 跳过已经是目标状态的任务
/// - 根据 `only_first_incomplete` 决定是处理一个还是全部
///
/// # 示例
///
/// ```ignore
/// let patches = build_todo_status_patches(&todos, "completed", false);
/// // patches 可能是: [{"id": "1", "status": "completed"}, {"id": "2", "status": "completed"}]
/// ```
pub(crate) fn build_todo_status_patches(
    items: &[todo::Todo],
    target_status: &str,
    only_first_incomplete: bool,
) -> Vec<serde_json::Value> {
    let mut out: Vec<serde_json::Value> = Vec::new();
    for t in items {
        // 跳过已完成的任务
        if t.status == "completed" {
            continue;
        }
        // 跳过已经是目标状态的任务
        if t.status == target_status {
            if only_first_incomplete {
                break;
            }
            continue;
        }
        // 添加状态更新补丁
        out.push(serde_json::json!({ "id": t.id, "status": target_status }));
        // 如果只需要第一个，则立即退出
        if only_first_incomplete {
            break;
        }
    }
    out
}

/// 尝试将第一个未开始的任务标记为进行中
///
/// 读取任务列表，找到第一个未完成且不是 "in_progress" 状态的任务，
/// 将其标记为 "in_progress"。此操作会通过工具执行器记录状态变更。
///
/// # 参数
///
/// - `session`: 会话实例的可变引用，用于执行工具
/// - `ctx`: 工具执行上下文，提供访问任务列表所需的配置和路径信息
/// - `tool_state`: 工具会话状态的可变引用，用于记录执行状态
///
/// # 行为说明
///
/// - 仅更新第一个符合条件的任务（`only_first_incomplete` = true）
/// - 使用静默模式执行（不触发事件回调）
/// - 如果读取、解析失败或没有需要更新的任务，则静默返回
///
/// # 副作用
///
/// - 会调用 `todowrite` 工具来更新任务状态
/// - 会更新 `tool_state`
///
/// # 示例
///
/// ```ignore
/// maybe_mark_todo_in_progress(&mut session, &ctx, &mut tool_state);
/// // 第一个未完成的任务现在可能被标记为 "in_progress"
/// ```
pub(crate) fn maybe_mark_todo_in_progress(
    session: &mut super::Session,
    ctx: &ToolRuntimeContext,
    tool_state: &mut super::ToolSessionState,
) {
    // 尝试读取原始任务列表数据，失败则静默返回
    let Ok(raw) = todo::read(ctx) else {
        return;
    };
    // 尝试解析为任务列表，失败则静默返回
    let Ok(items) = serde_json::from_str::<Vec<todo::Todo>>(raw.trim()) else {
        return;
    };
    // 构建将第一个未完成任务标记为进行中的状态补丁
    let patches = build_todo_status_patches(&items, "in_progress", true);
    // 如果没有需要更新的任务，静默返回
    if patches.is_empty() {
        return;
    }
    // 构建工具输入参数：任务补丁列表和合并标志
    let input = serde_json::json!({ "todos": patches, "merge": true }).to_string();
    // 创建静默事件处理器（忽略所有事件）
    let mut noop = |_ev: StreamEvent| true;
    // 执行 todowrite 工具并记录结果，使用静默模式（false 表示不记录到历史）
    let _ = super::tools_exec::run_tool_and_record(
        session,
        "todowrite",
        &input,
        ctx,
        false,
        &mut noop,
        tool_state,
    );
}
#[cfg(test)]
#[path = "todos_tests.rs"]
mod todos_tests;
