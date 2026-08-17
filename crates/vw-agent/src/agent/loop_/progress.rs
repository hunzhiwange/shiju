//! 进度显示与格式化模块
//!
//! 本模块负责处理代理执行过程中的进度信息展示，主要功能包括：
//! - 定义进度更新的时间间隔控制，防止消息洪泛
//! - 提供草稿通道的哨兵值（sentinel），用于信号传递和状态管理
//! - 格式化工具调用的进度信息，使其适合在用户界面显示
//! - 从工具参数中提取关键信息，生成简洁的进度提示文本
//!
//! # 设计目标
//!
//! 1. **节流控制**: 通过最小时间间隔限制，避免过于频繁的进度更新
//! 2. **通道通信**: 使用哨兵值在组件间传递特殊信号（清空、进度等）
//! 3. **用户友好**: 将复杂的工具调用参数转换为易读的进度标签
//!
//! # 使用场景
//!
//! - 在代理执行工具时，向用户展示当前正在执行的操作
//! - 在流式响应中，区分内部进度消息和最终输出
//! - 在通道层（如 Telegram、Discord）中，根据用户偏好决定是否显示详细进度

use crate::app::agent::util::truncate_with_ellipsis;

/// 进度发送的最小时间间隔（毫秒）
///
/// 该常量定义了两次进度更新之间的最小时间间隔，用于防止消息洪泛（flooding）
/// 草稿通道（draft channel）。过快的更新可能导致：
/// - 通道拥塞，影响其他消息的传递
/// - 用户界面频繁刷新，降低用户体验
/// - 不必要的资源消耗
///
/// # 默认值
///
/// 500 毫秒，即每秒最多发送 2 次进度更新
///
/// # 使用示例
///
/// ```ignore
/// use std::time::{Duration, Instant};
///
/// let mut last_progress = Instant::now();
/// let min_interval = Duration::from_millis(PROGRESS_MIN_INTERVAL_MS);
///
/// if last_progress.elapsed() >= min_interval {
///     send_progress_update();
///     last_progress = Instant::now();
/// }
/// ```
pub(crate) const PROGRESS_MIN_INTERVAL_MS: u64 = 500;

/// 草稿清空哨兵值
///
/// 该哨兵值通过 `on_delta` 通道发送，用于通知草稿更新器清空累积的文本。
/// 主要用于在流式传输最终答案之前，清除之前的进度行，使其被干净的响应替换。
///
/// # 工作原理
///
/// 1. 代理在执行过程中可能会累积多行进度信息（如工具调用状态）
/// 2. 当准备输出最终答案时，发送此哨兵值
/// 3. 接收端识别到此哨兵后，清空之前的累积文本
/// 4. 随后流式传输的最终答案将完全替换进度显示
///
/// # 格式
///
/// 使用特殊的 NUL 字符（`\x00`）包裹，确保不会与正常文本混淆
///
/// # 使用示例
///
/// ```ignore
/// // 在发送最终答案之前清空草稿
/// on_delta.send(DRAFT_CLEAR_SENTINEL.to_string()).await?;
///
/// // 然后开始流式传输最终答案
/// for chunk in final_answer_chunks {
///     on_delta.send(chunk).await?;
/// }
/// ```
pub const DRAFT_CLEAR_SENTINEL: &str = "\x00CLEAR\x00";

/// 草稿进度哨兵值前缀
///
/// 该哨兵值用于标记内部进度增量消息（如思考过程、工具执行追踪）。
/// 通道层可以根据该前缀识别并默认抑制这些消息，仅在用户明确要求查看
/// 命令/工具执行详情时才显示。
///
/// # 设计目的
///
/// 1. **信息分层**: 区分用户关心的最终输出和内部执行细节
/// 2. **默认简洁**: 默认情况下隐藏技术细节，保持界面简洁
/// 3. **可选透明**: 用户可以选择性地查看完整的执行过程
///
/// # 格式
///
/// 使用特殊的 NUL 字符（`\x00`）包裹，确保不会与正常文本混淆
///
/// # 使用示例
///
/// ```ignore
/// // 在通道层处理消息时
/// if message.starts_with(DRAFT_PROGRESS_SENTINEL) {
///     // 这是内部进度消息
///     if user_wants_detailed_view {
///         // 用户明确要求查看详情，显示消息
///         display_message(&message[DRAFT_PROGRESS_SENTINEL.len()..]);
///     } else {
///         // 默认情况下跳过
///         return;
///     }
/// }
/// ```
pub const DRAFT_PROGRESS_SENTINEL: &str = "\x00PROGRESS\x00";

/// WebSocket 私有结构化事件哨兵值前缀。
///
/// 该哨兵值必须附着在 [`DRAFT_PROGRESS_SENTINEL`] 之后使用，
/// 专供 WebSocket 网关从内部进度流中提取结构化事件，其他渠道应忽略其可见内容，
/// 以避免把原始 JSON 直接展示给终端、IM 或邮件等用户界面。
pub(crate) const DRAFT_WS_EVENT_SENTINEL: &str = "\x00WS_EVENT\x00";

/// 从工具调用参数中提取简短提示文本，用于进度显示
///
/// 该函数根据工具名称，从 JSON 参数中提取最相关的字段，
/// 并将其截断为适合在进度界面显示的简短文本。
///
/// # 参数
///
/// - `name`: 工具名称（如 "shell"、"file_read"、"file_write" 等）
/// - `args`: 工具调用的参数，以 JSON 值形式提供
/// - `max_len`: 提示文本的最大长度（字符数），超出部分将被截断
///
/// # 返回值
///
/// 返回提取并格式化后的提示文本字符串。如果无法提取有效提示，
/// 则返回空字符串。
///
/// # 提取策略
///
/// 不同工具会从不同字段提取提示信息：
///
/// - `shell`: 从 `command` 字段提取命令内容
/// - `file_read` / `file_write` / `edit`: 从路径字段提取文件路径
/// - 其他工具: 依次尝试 `action` 字段和 `query` 字段
///
/// # 使用示例
///
/// ```ignore
/// use serde_json::json;
///
/// // Shell 命令示例
/// let args = json!({"command": "ls -la /tmp/very/long/path"});
/// let hint = truncate_tool_args_for_progress("shell", &args, 30);
/// assert_eq!(hint, "ls -la /tmp/very/long/p...");
///
/// // 文件读取示例
/// let args = json!({"path": "/etc/config.yaml"});
/// let hint = truncate_tool_args_for_progress("file_read", &args, 20);
/// assert_eq!(hint, "/etc/config.yaml");
///
/// // 无有效字段示例
/// let args = json!({"other": "value"});
/// let hint = truncate_tool_args_for_progress("unknown", &args, 20);
/// assert_eq!(hint, "");
/// ```
pub(crate) fn truncate_tool_args_for_progress(
    name: &str,
    args: &serde_json::Value,
    max_len: usize,
) -> String {
    // 根据工具名称选择要提取的字段
    // 不同工具的参数结构不同，需要针对性地提取最相关的信息
    let hint = match name {
        // shell 工具：提取执行的命令内容
        "shell" => args.get("command").and_then(|v| v.as_str()),
        // 文件操作工具：提取文件路径
        "file_read" | "file_write" => args.get("path").and_then(|v| v.as_str()),
        "file_edit" => args
            .get("file_path")
            .and_then(|v| v.as_str())
            .or_else(|| args.get("filePath").and_then(|v| v.as_str()))
            .or_else(|| args.get("path").and_then(|v| v.as_str())),
        // 其他工具：依次尝试 action 和 query 字段
        _ => args
            .get("action")
            .and_then(|v| v.as_str())
            .or_else(|| args.get("query").and_then(|v| v.as_str())),
    };

    // 如果提取到了提示文本，则截断到指定长度；否则返回空字符串
    match hint {
        Some(s) => truncate_with_ellipsis(s, max_len),
        None => String::new(),
    }
}

/// 获取工具进度的动作标签（开始/完成）
///
/// 该函数根据工具名称返回用于进度显示的"正在执行"和"执行完毕"标签。
/// 不同类型的工具可以使用不同的表述，使其更符合操作语义。
///
/// # 参数
///
/// - `tool_name`: 工具名称
///
/// # 返回值
///
/// 返回一个元组 `(开始标签, 完成标签)`，包含两个静态字符串引用：
/// - 第一个元素：表示"正在执行中"的状态文本
/// - 第二个元素：表示"执行完毕"的状态文本
///
/// # 工具类型与标签映射
///
/// - `file_read` / `read`: 使用"读取中"和"执行完毕"
/// - 其他工具: 使用通用的"执行中"和"执行完毕"
///
/// # 使用示例
///
/// ```ignore
/// // 文件读取工具
/// let (in_progress, completed) = tool_progress_actions("file_read");
/// assert_eq!(in_progress, "读取中");
/// assert_eq!(completed, "执行完毕");
///
/// // Shell 工具
/// let (in_progress, completed) = tool_progress_actions("shell");
/// assert_eq!(in_progress, "执行中");
/// assert_eq!(completed, "执行完毕");
///
/// // 在进度显示中使用
/// println!("{}...", in_progress);
/// // ... 工具执行 ...
/// println!("{}!", completed);
/// ```
pub(crate) fn tool_progress_actions(tool_name: &str) -> (&'static str, &'static str) {
    // 文件读取操作使用更具体的"读取中"标签
    if matches!(tool_name, "file_read" | "read") {
        ("读取中", "执行完毕")
    } else {
        // 其他所有工具使用通用的"执行中"标签
        ("执行中", "执行完毕")
    }
}

/// 生成工具进度的完整标签文本
///
/// 该函数根据工具名称和参数，生成用于进度显示的完整标签。
/// 标签包含工具名称和关键参数信息，格式化后适合在用户界面展示。
///
/// # 参数
///
/// - `tool_name`: 工具名称（如 "file_read"、"shell" 等）
/// - `args`: 工具调用的参数，以 JSON 值形式提供
///
/// # 返回值
///
/// 返回格式化后的进度标签字符串，包含工具名称和相关参数提示。
///
/// # 格式化策略
///
/// ## 文件读取工具 (`file_read` / `read`)
///
/// 生成详细的文件读取标签，包含：
/// - 文件路径
/// - 读取范围（offset 和 limit，如果指定）
///
/// 格式示例：
/// - `/path/to/file` - 完整文件读取
/// - `/path/to/file [offset=100, limit=50]` - 带范围读取
/// - `文件 [limit=100]` - 未知路径时
///
/// ## 其他工具
///
/// 生成通用标签，包含：
/// - 工具名称
/// - 从参数中提取的简短提示（如果有）
///
/// 格式示例：
/// - `tool_name` - 无参数提示
/// - `tool_name: arg_hint...` - 带参数提示
///
/// # 使用示例
///
/// ```ignore
/// use serde_json::json;
///
/// // 完整文件读取
/// let args = json!({"path": "/etc/hosts"});
/// let label = tool_progress_label("file_read", &args);
/// assert_eq!(label, "/etc/hosts");
///
/// // 带范围的文件读取
/// let args = json!({
///     "path": "/var/log/app.log",
///     "offset": 1000,
///     "limit": 100
/// });
/// let label = tool_progress_label("file_read", &args);
/// assert_eq!(label, "/var/log/app.log [offset=1000, limit=100]");
///
/// // Shell 命令
/// let args = json!({"command": "npm install"});
/// let label = tool_progress_label("shell", &args);
/// assert_eq!(label, "shell: npm install");
///
/// // 无参数提示的工具
/// let args = json!({});
/// let label = tool_progress_label("custom_tool", &args);
/// assert_eq!(label, "custom_tool");
/// ```
pub(crate) fn tool_progress_label(tool_name: &str, args: &serde_json::Value) -> String {
    // 对文件读取工具使用特殊格式，包含路径和范围信息
    if matches!(tool_name, "file_read" | "read") {
        // 提取文件路径，如果不存在则使用空字符串
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");

        // 提取读取范围参数
        let offset = args.get("offset").and_then(|v| v.as_i64());
        let limit = args.get("limit").and_then(|v| v.as_i64());

        // 构建范围信息字符串，仅展示实际提供的参数
        let mut parts = Vec::new();
        if let Some(offset) = offset {
            parts.push(format!("offset={}", offset.max(1)));
        }
        if let Some(limit) = limit {
            parts.push(format!("limit={limit}"));
        }
        let range =
            if parts.is_empty() { String::new() } else { format!(" [{}]", parts.join(", ")) };

        // 如果路径为空，显示"文件"作为通用描述；否则显示完整路径
        if path.is_empty() { format!("文件{range}") } else { format!("{path}{range}") }
    } else {
        // 对其他工具，生成"工具名: 参数提示"格式的标签
        // 从参数中提取最多 60 个字符的提示信息
        let hint = truncate_tool_args_for_progress(tool_name, args, 60);

        // 如果没有提取到提示信息，仅显示工具名称；否则显示完整标签
        if hint.is_empty() { tool_name.to_string() } else { format!("{tool_name}: {hint}") }
    }
}

#[cfg(test)]
#[path = "progress_tests.rs"]
mod progress_tests;
