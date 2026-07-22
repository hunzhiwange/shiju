//! 规划处理器模块
//!
//! 本模块提供规划答案的处理功能，专门用于从代理的规划响应中提取和执行待办事项（todo）管理操作。
//!
//! # 主要功能
//!
//! - 解析规划响应中的 `todowrite` 工具调用并执行
//! - 当规划文本中未包含显式工具调用时，自动从文本中识别待办事项
//! - 支持多种常见的待办事项列表格式（Markdown 任务列表、有序列表、无序列表等）
//!
//! # 工作流程
//!
//! 1. 解析规划答案文本，查找工具调用模式（如 `/todowrite`）
//! 2. 如果找到 `todowrite` 调用，直接执行
//! 3. 如果未找到但文本包含待办事项列表，自动提取并生成待办事项
//!
//! # 使用场景
//!
//! 此模块主要用于规划阶段，当代理返回包含任务列表的响应时，
//! 确保这些任务被正确记录到待办事项系统中。

use super::types::StreamEvent;
use crate::app::agent::tools::ToolRuntimeContext;
use crate::app::agent::tools::{TODO_WRITE_TOOL_ID, is_todo_write_tool_id};
use std::collections::HashSet;

/// 从规划答案中解析并执行 `todowrite` 工具调用
///
/// 此函数处理规划阶段的答案，专门提取和执行 `todowrite` 工具调用。
/// 它会遍历答案的每一行，查找符合工具调用模式的文本，并执行找到的 `todowrite` 调用。
///
/// # 参数
///
/// * `session` - 会话实例，用于记录工具调用历史和执行上下文（可变引用）
/// * `answer` - 规划答案文本，可能包含工具调用或待办事项列表
/// * `ctx` - 工具执行上下文，提供执行环境信息
/// * `allowed_tools` - 允许执行的工具集合，用于过滤不合法的工具调用
/// * `on_event` - 事件回调函数，用于处理流式事件（如进度更新、错误等）
/// * `tool_state` - 工具会话状态，用于跟踪工具执行的持久化状态（可变引用）
///
/// # 返回值
///
/// 返回 `bool` 类型：
/// - `true` - 成功执行了至少一次 `todowrite` 工具调用
/// - `false` - 未执行任何 `todowrite` 调用
///
/// # 执行逻辑
///
/// 1. **解析阶段**：逐行扫描答案，查找工具调用模式
/// 2. **直接执行**：如果找到显式的 `todowrite` 调用，立即执行
/// 3. **自动提取**：如果未找到显式调用但允许使用 `todowrite`，尝试从文本中提取待办事项
///
/// # 示例
///
/// ```ignore
/// let mut session = Session::new();
/// let ctx = Context::default();
/// let allowed_tools = HashSet::from(["todowrite"]);
/// let mut tool_state = ToolSessionState::default();
///
/// // 处理包含待办事项的规划答案
/// let result = ingest_planning_answer_todowrite_only(
///     &mut session,
///     "1. 实现登录功能\n2. 添加用户验证\n3. 编写测试用例",
///     &ctx,
///     &allowed_tools,
///     &mut |event| true,
///     &mut tool_state,
/// );
///
/// assert!(result); // 成功提取并执行了 todowrite
/// ```
pub(crate) fn ingest_planning_answer_todowrite_only(
    session: &mut super::Session,
    answer: &str,
    ctx: &ToolRuntimeContext,
    allowed_tools: &HashSet<String>,
    on_event: &mut dyn FnMut(StreamEvent) -> bool,
    tool_state: &mut super::ToolSessionState,
) -> bool {
    // 将答案按行分割，便于逐行解析工具调用
    let lines: Vec<&str> = answer.lines().collect();

    // 当前行索引，用于遍历所有行
    let mut i = 0usize;

    // 标记是否已执行过 todowrite 工具
    let mut ran_todowrite = false;

    // 遍历所有行，查找并执行工具调用
    while i < lines.len() {
        // 尝试在当前位置解析工具调用
        // parse_tool_at 返回工具名称、输入参数和消耗的行数
        if let Some((name, input, consumed)) = super::utils::parse_tool_at(&lines, i, allowed_tools)
        {
            // 只处理 todowrite 工具，忽略其他工具调用
            if is_todo_write_tool_id(&name) {
                ran_todowrite = true;

                // 构建工具调用字符串（格式：/todowrite 或 /todowrite <参数>）
                let call = if input.trim().is_empty() {
                    format!("/{}", name)
                } else {
                    format!("/{} {}", name, input.trim())
                };

                // 将工具调用记录到会话历史中
                session.push(super::Role::Assistant, call);

                // 执行工具调用并记录结果
                let mut forward_event = |event| on_event(event);
                let _ = super::tools_exec::run_tool_and_record(
                    session,
                    &name,
                    &input,
                    ctx,
                    true,
                    &mut forward_event,
                    tool_state,
                );
            }

            // 跳过已解析的行（consumed 可能大于 1，因为工具调用可能跨多行）
            i += consumed;
            continue;
        }
        i += 1;
    }

    // 如果未找到显式的 todowrite 调用，但该工具在允许列表中，
    // 则尝试从文本中自动提取待办事项
    if !ran_todowrite && allowed_tools.iter().any(|name| is_todo_write_tool_id(name)) {
        if let Some(input) = build_todowrite_from_text(answer) {
            // 记录隐式的 todowrite 调用
            session.push(super::Role::Assistant, format!("/{TODO_WRITE_TOOL_ID}"));

            // 执行自动提取的待办事项
            let mut forward_event = |event| on_event(event);
            let _ = super::tools_exec::run_tool_and_record(
                session,
                TODO_WRITE_TOOL_ID,
                &input,
                ctx,
                true,
                &mut forward_event,
                tool_state,
            );
            ran_todowrite = true;
        }
    }

    ran_todowrite
}

/// 从文本中自动提取待办事项并构建 `todowrite` 工具的 JSON 输入
///
/// 此函数解析文本内容，识别多种常见的列表格式，提取出待办事项，
/// 并将其转换为 `todowrite` 工具所需的 JSON 格式。
///
/// # 支持的列表格式
///
/// - Markdown 任务列表：`- [ ]` 或 `- [x]` 或 `- [X]`
/// - 无序列表：`-` 或 `*` 开头
/// - 有序列表：`1.` 或 `1)` 格式的数字编号
///
/// # 参数
///
/// * `answer` - 待解析的文本内容
///
/// # 返回值
///
/// 返回 `Option<String>`：
/// - `Some(String)` - 成功提取到待办事项，返回 JSON 格式的字符串
/// - `None` - 未找到有效的待办事项
///
/// # 提取规则
///
/// 1. **跳过代码块**：忽略 ``` 包裹的代码块内容
/// 2. **去重处理**：基于小写标准化后的文本进行去重
/// 3. **长度限制**：忽略过短（少于 2 字符）的行
/// 4. **数量限制**：最多提取 12 个待办事项
/// 5. **格式清理**：移除列表标记、数字编号等前缀
///
/// # 输出格式
///
/// ```json
/// {
///   "todos": [
///     {
///       "id": "1",
///       "content": "实现登录功能",
///       "status": "pending",
///       "priority": "medium"
///     }
///   ],
///   "merge": false
/// }
/// ```
///
/// # 示例
///
/// ```ignore
/// // 输入包含多种格式的列表
/// let text = r#"
/// 1. 实现用户认证
/// 2. 添加日志记录
/// - [ ] 编写单元测试
/// - 代码审查
/// "#;
///
/// let result = build_todowrite_from_text(text);
/// assert!(result.is_some());
///
/// let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
/// assert_eq!(json["todos"].as_array().unwrap().len(), 4);
/// ```
fn build_todowrite_from_text(answer: &str) -> Option<String> {
    // 标记是否在代码块内（代码块内容应跳过）
    let mut in_code = false;

    // 提取的待办事项列表
    let mut items: Vec<String> = Vec::new();

    // 已见过的待办事项（用于去重，存储小写标准化后的文本）
    let mut seen = std::collections::HashSet::<String>::new();

    // 逐行解析文本
    for raw in answer.lines() {
        let mut line = raw.trim();

        // 检测代码块边界（```）
        if line.starts_with("```") {
            in_code = !in_code;
            continue;
        }

        // 跳过代码块内的所有内容
        if in_code {
            continue;
        }

        // 移除 Markdown 任务列表标记：- [ ] 或 - [x] 或 - [X]
        if let Some(rest) = line.strip_prefix("- [ ]") {
            line = rest.trim();
        } else if let Some(rest) = line.strip_prefix("- [x]") {
            line = rest.trim();
        } else if let Some(rest) = line.strip_prefix("- [X]") {
            line = rest.trim();
        }

        // 移除无序列表标记：- 或 *
        line = line.strip_prefix('-').or_else(|| line.strip_prefix('*')).unwrap_or(line).trim();

        // 移除数字编号加点格式（如 "1. ", "2. " 等）
        if let Some(dot) = line.find('.') {
            let (head, tail) = line.split_at(dot);
            // 只有当点前面全是数字时才认为是编号
            if !head.is_empty() && head.chars().all(|c| c.is_ascii_digit()) {
                line = tail.trim_start_matches('.').trim();
            }
        }

        // 移除数字编号加括号格式（如 "1) ", "2) " 等）
        if let Some(paren) = line.find(')') {
            let (head, tail) = line.split_at(paren);
            // 只有当括号前面全是数字时才认为是编号
            if !head.is_empty() && head.chars().all(|c| c.is_ascii_digit()) {
                line = tail.trim_start_matches(')').trim();
            }
        }

        // 跳过空行或以 / 开头的行（可能是命令）
        if line.is_empty() || line.starts_with('/') {
            continue;
        }

        // 跳过过短的行（少于 2 个字符）
        if line.len() < 2 {
            continue;
        }

        // 标准化空白字符：将多个连续空格/制表符压缩为单个空格
        let normalized = line.split_whitespace().collect::<Vec<_>>().join(" ");

        // 生成去重用的键（小写形式）
        let key = normalized.to_lowercase();

        // 如果已存在相同的待办事项，跳过
        if !seen.insert(key) {
            continue;
        }

        // 添加到待办事项列表
        items.push(normalized);

        // 限制最大数量为 12 个待办事项
        if items.len() >= 12 {
            break;
        }
    }

    // 如果未提取到任何待办事项，返回 None
    if items.is_empty() {
        return None;
    }

    // 构建 JSON 格式的待办事项列表
    let todos = items
        .into_iter()
        .enumerate()
        .map(|(i, content)| {
            serde_json::json!({
                "id": (i + 1).to_string(),  // ID 从 1 开始
                "content": content,          // 待办事项内容
                "status": "pending",         // 默认状态为待处理
                "priority": "medium",        // 默认优先级为中等
            })
        })
        .collect::<Vec<_>>();

    // 构建最终的 JSON 对象并转换为字符串
    // merge: false 表示替换而非合并现有待办事项
    Some(serde_json::json!({ "todos": todos, "merge": false }).to_string())
}
#[cfg(test)]
#[path = "planning_tests.rs"]
mod planning_tests;
