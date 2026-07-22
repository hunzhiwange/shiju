//! Todo 卡片 CLI 显示模块
//!
//! 本模块提供 CLI 转录界面中待办事项（Todo）卡片的解析与显示支持。
//! 主要功能包括：
//! - 工具徽章的颜色映射（为不同类型工具分配可区分的显示颜色）
//! - 待办事项状态符号的转换
//! - 从 JSON 数据中解析待办事项列表
//!
//! 该模块与 `ratatui` 库集成，用于终端 UI 的样式渲染。

use super::super::theme::{ACCENT_CYAN, SUCCESS, TEXT_SUBTLE, WARNING};
use ratatui::style::Color;

/// 根据工具名称返回 CLI 显示用的徽章文本与颜色
///
/// 该函数将工具名称映射到简短的徽章标识符和对应的颜色，
/// 以便在 CLI 界面中快速区分不同类型的工具调用。
///
/// # 参数
///
/// * `tool_name` - 工具名称字符串，如 "read"、"bash"、"todowrite" 等
///
/// # 返回值
///
/// 返回一个元组，包含：
/// - `&'static str`: 徽章文本标识（如 "[R]"、"[B]" 等）
/// - `Color`: 对应的 `ratatui` 颜色值
///
/// # 映射规则
///
/// | 工具类型 | 徽章 | 颜色 |
/// |---------|------|------|
/// | 文件读取 (`read`) | [R] | 绿色 |
/// | 文件编辑 (`write`, `apply_patch`) | [E] | 黄色 |
/// | 搜索类 (`grep`, `glob`, `lsp`，含旧别名) | [S] | 蓝色 |
/// | Shell 执行 (`bash`) | [B] | 洋红色 |
/// | 待办管理 (`todowrite`, `todoread`) | [T] | 青色 |
/// | 其他未知工具 | [U] | 深灰色 |
///
/// # 示例
///
/// ```ignore
/// let (badge, color) = tool_badge_cli("bash");
/// assert_eq!(badge, "[B]");
/// assert_eq!(color, Color::Magenta);
///
/// let (badge, color) = tool_badge_cli("unknown_tool");
/// assert_eq!(badge, "[U]");
/// ```
pub(crate) fn tool_badge_cli(tool_name: &str) -> (&'static str, Color) {
    match tool_name {
        "read" => ("[R]", SUCCESS),
        "write" | "apply_patch" => ("[E]", WARNING),
        "grep" | "content_search" | "glob" | "glob_search" | "lsp" | "codesearch" => {
            ("[S]", Color::Rgb(118, 176, 255))
        }
        "bash" | "shell" => ("[B]", Color::Rgb(216, 150, 255)),
        "todowrite" | "todoread" => ("[T]", ACCENT_CYAN),
        _ => ("[U]", TEXT_SUBTLE),
    }
}

/// 将待办事项状态转换为显示符号
///
/// 该函数将待办事项的状态字符串映射为简洁的 Unicode 符号，
/// 用于在 CLI 界面中以视觉方式表示任务进度。
///
/// # 参数
///
/// * `status` - 状态字符串，预期值为 "completed"、"in_progress" 或其他
///
/// # 返回值
///
/// - `"✓"` - 已完成状态（`completed`）
/// - `"·"` - 进行中状态（`in_progress`）
/// - `"○"` - 其他状态（通常为待处理 `pending`）
///
/// # 示例
///
/// ```ignore
/// assert_eq!(todo_status_symbol("completed"), "✓");
/// assert_eq!(todo_status_symbol("in_progress"), "·");
/// assert_eq!(todo_status_symbol("pending"), "○");
/// assert_eq!(todo_status_symbol("cancelled"), "○");
/// ```
pub(crate) fn todo_status_symbol(status: &str) -> &'static str {
    match status {
        "completed" => "✓",
        "in_progress" => "·",
        _ => "○",
    }
}

/// 待办事项卡片数据结构
///
/// 该结构体存储待办事项列表的汇总统计信息和详细条目，
/// 用于在 CLI 转录界面中渲染待办事项卡片。
///
/// # 字段说明
///
/// * `total` - 待办事项总数
/// * `done` - 已完成事项数量
/// * `running` - 进行中事项数量
/// * `pending` - 待处理事项数量
/// * `items` - 待办事项详情列表，每项包含 (状态, 内容) 元组
///
/// # 不变性
///
/// - `total` 应等于 `done + running + pending`
/// - 所有计数字段应使用 `saturating_add` 进行累加以避免溢出
#[derive(Default)]
pub(crate) struct TodoCardData {
    pub(crate) total: usize,
    pub(crate) done: usize,
    pub(crate) running: usize,
    pub(crate) pending: usize,
    pub(crate) items: Vec<(String, String)>,
}

/// 从 JSON 数组解析待办事项列表
///
/// 该函数遍历 JSON 数组中的每个元素，提取状态和内容字段，
/// 并统计各状态类别的数量。
///
/// # 参数
///
/// * `arr` - JSON 值数组，每个元素应包含 `status` 和 `content` 字段
///
/// # 返回值
///
/// 返回填充完成的 `TodoCardData` 实例，包含：
/// - 各状态的计数统计
/// - 所有待办事项的 (状态, 内容) 列表
///
/// # 容错处理
///
/// - 若 `status` 字段缺失或非字符串，默认使用 "pending"
/// - 若 `content` 字段缺失或非字符串，默认使用 "(empty)"
/// - 对字符串值执行 `trim()` 以去除首尾空白
/// - 所有计数使用 `saturating_add` 防止溢出
///
/// # 示例
///
/// ```ignore
/// use serde_json::json;
///
/// let arr = vec![
///     json!({"status": "completed", "content": "Task 1"}),
///     json!({"status": "in_progress", "content": "Task 2"}),
/// ];
/// let data = parse_todos_from_array(&arr);
/// assert_eq!(data.total, 2);
/// assert_eq!(data.done, 1);
/// assert_eq!(data.running, 1);
/// ```
pub(crate) fn parse_todos_from_array(arr: &[serde_json::Value]) -> TodoCardData {
    let mut out = TodoCardData::default();

    for item in arr {
        // 提取状态字段，缺失或类型不匹配时默认为 "pending"
        let status =
            item.get("status").and_then(|x| x.as_str()).unwrap_or("pending").trim().to_string();

        // 提取内容字段，缺失或类型不匹配时默认为 "(empty)"
        let content =
            item.get("content").and_then(|x| x.as_str()).unwrap_or("(empty)").trim().to_string();

        // 根据状态更新对应计数器
        match status.as_str() {
            "completed" => out.done = out.done.saturating_add(1),
            "in_progress" => out.running = out.running.saturating_add(1),
            _ => out.pending = out.pending.saturating_add(1),
        }

        // 记录条目并更新总数
        out.items.push((status, content));
        out.total = out.total.saturating_add(1);
    }

    out
}

/// 从 JSON 值或 JSON 字符串中解析出 JSON 值
///
/// 该函数处理两种可能的输入格式：
/// 1. 直接的 JSON 字符串：尝试将其解析为 JSON 值
/// 2. 已经是 JSON 值：直接克隆返回
///
/// # 参数
///
/// * `v` - 待解析的 JSON 值引用
///
/// # 返回值
///
/// - `Some(JsonValue)` - 解析成功，返回 JSON 值
/// - `None` - 输入为字符串但解析失败（无效 JSON）
///
/// # 使用场景
///
/// 该函数用于处理 API 响应中可能出现的双重编码情况，
/// 即 JSON 字符串被再次序列化为 JSON 字符串的场景。
///
/// # 示例
///
/// ```ignore
/// use serde_json::json;
///
/// // 字符串形式的 JSON
/// let v = json!("[{\"status\": \"completed\"}]");
/// let parsed = parse_json_from_value_or_json_string(&v);
/// assert!(parsed.is_some());
///
/// // 直接的 JSON 对象
/// let v = json!({"key": "value"});
/// let parsed = parse_json_from_value_or_json_string(&v);
/// assert!(parsed.is_some());
/// ```
pub(crate) fn parse_json_from_value_or_json_string(
    v: &serde_json::Value,
) -> Option<serde_json::Value> {
    // 若值为字符串，尝试将其内容解析为 JSON
    if let Some(s) = v.as_str() {
        return serde_json::from_str::<serde_json::Value>(s.trim()).ok();
    }
    // 否则直接克隆该 JSON 值
    Some(v.clone())
}

/// 解析 `todowrite` 工具调用的卡片数据
///
/// 该函数从 `todowrite` 工具的原始 JSON 输入字符串中提取待办事项列表。
/// 支持处理输入字段可能是 JSON 字符串的情况（双重编码）。
///
/// # 参数
///
/// * `input_raw` - 原始 JSON 字符串，预期结构为 `{"input": {"todos": [...]}}` 或
///   `{"input": "{...JSON字符串...}"}`
///
/// # 返回值
///
/// - `Some(TodoCardData)` - 解析成功，返回待办事项数据
/// - `None` - 解析失败（JSON 格式错误、缺少必要字段、类型不匹配等）
///
/// # JSON 结构期望
///
/// ```json
/// {
///     "input": {
///         "todos": [
///             {"status": "completed", "content": "完成任务A"},
///             {"status": "in_progress", "content": "执行任务B"}
///         ]
///     }
/// }
/// ```
///
/// # 示例
///
/// ```ignore
/// use serde_json::json;
///
/// let input = json!({
///     "input": {
///         "todos": [
///             {"status": "completed", "content": "Task 1"}
///         ]
///     }
/// }).to_string();
///
/// let data = parse_todowrite_card_data(&input);
/// assert!(data.is_some());
/// assert_eq!(data.unwrap().done, 1);
/// ```
pub(crate) fn parse_todowrite_card_data(input_raw: &str) -> Option<TodoCardData> {
    // 解析原始 JSON 字符串为根对象
    let root = serde_json::from_str::<serde_json::Value>(input_raw.trim()).ok()?;

    // 获取 input 字段，处理可能的 JSON 字符串双重编码
    let input = parse_json_from_value_or_json_string(root.get("input")?)?;

    // 获取 todos 数组并解析
    let todos = input.get("todos")?.as_array()?;

    Some(parse_todos_from_array(todos))
}

/// 解析 `todoread` 工具调用的卡片数据
///
/// 该函数从 `todoread` 工具的原始 JSON 输出字符串中提取待办事项列表。
/// 与 `parse_todowrite_card_data` 不同，此函数从 `output` 字段读取数据。
///
/// # 参数
///
/// * `input_raw` - 原始 JSON 字符串，预期结构为 `{"output": [...]}` 或
///   `{"output": "[...JSON字符串...]"}`
///
/// # 返回值
///
/// - `Some(TodoCardData)` - 解析成功，返回待办事项数据
/// - `None` - 解析失败（JSON 格式错误、缺少必要字段、类型不匹配等）
///
/// # JSON 结构期望
///
/// ```json
/// {
///     "output": [
///         {"status": "completed", "content": "完成任务A"},
///         {"status": "pending", "content": "待完成任务B"}
///     ]
/// }
/// ```
///
/// # 容错处理
///
/// - 支持 `output` 字段直接为数组
/// - 支持 `output` 字段为 JSON 字符串（需进一步解析）
/// - 自动去除字符串首尾空白
///
/// # 示例
///
/// ```ignore
/// use serde_json::json;
///
/// let input = json!({
///     "output": [
///         {"status": "completed", "content": "Task 1"},
///         {"status": "pending", "content": "Task 2"}
///     ]
/// }).to_string();
///
/// let data = parse_todoread_card_data(&input);
/// assert!(data.is_some());
/// let data = data.unwrap();
/// assert_eq!(data.total, 2);
/// assert_eq!(data.done, 1);
/// assert_eq!(data.pending, 1);
/// ```
pub(crate) fn parse_todoread_card_data(input_raw: &str) -> Option<TodoCardData> {
    // 解析原始 JSON 字符串为根对象
    let root = serde_json::from_str::<serde_json::Value>(input_raw.trim()).ok()?;

    // 获取 output 字段
    let output = root.get("output")?;

    // 处理 output 可能是字符串或直接数组的情况
    let parsed = if let Some(s) = output.as_str() {
        // output 为字符串，需要进一步解析为 JSON
        serde_json::from_str::<serde_json::Value>(s.trim()).ok()?
    } else {
        // output 已经是 JSON 值，直接使用
        output.clone()
    };

    // 获取数组并解析待办事项
    let arr = parsed.as_array()?;

    Some(parse_todos_from_array(arr))
}
