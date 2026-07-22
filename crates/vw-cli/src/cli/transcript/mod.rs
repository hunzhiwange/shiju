//! CLI 对话记录（Transcript）渲染模块
//!
//! 本模块负责管理 CLI 界面中的对话记录显示，包括消息解析、格式化和样式化。
//! 它将原始的对话数据转换为 ratatui 可渲染的组件。
//!
//! # 核心功能
//!
//! - **对话条目管理**：通过 [`TranscriptEntry`] 和 [`TranscriptRole`] 定义对话的结构和角色
//! - **消息解析**：将助手消息解析为结构化片段（文本、思考块、工具调用）
//! - **样式化显示**：为不同角色提供差异化的视觉样式
//! - **摘要生成**：将完整对话记录转换为可滚动的行视图
//!
//! # 子模块
//!
//! - [`assistant_segments`]：解析助手消息中的文本、`<think/>` 块和工具调用
//! - [`structured_stream`]：构建流式输出的结构化视图
//! - [`summary`]：将对话记录转换为可渲染的行列表
//! - [`todocards`]：渲染 todo 工具调用的卡片式展示
//! - [`truncate`]：文本截断工具，用于限制显示长度
//!
//! # 使用示例
//!
//! ```ignore
//! use transcript::{TranscriptEntry, TranscriptRole, transcript_to_lines};
//!
//! // 创建对话条目
//! let entry = TranscriptEntry::new(TranscriptRole::User, "Hello, agent!");
//!
//! // 转换为可渲染行
//! let (lines, meta) = transcript_to_lines(&[entry], false, false, &Default::default(), "");
//! ```

mod assistant_segments;
#[cfg(test)]
#[path = "assistant_segments_tests.rs"]
mod assistant_segments_tests;
mod structured_stream;
#[cfg(test)]
#[path = "structured_stream_tests.rs"]
mod structured_stream_tests;
mod summary;
#[cfg(test)]
#[path = "summary_tests.rs"]
mod summary_tests;
mod todocards;
#[cfg(test)]
#[path = "todocards_tests.rs"]
mod todocards_tests;
mod toolcards;
#[cfg(test)]
#[path = "toolcards_tests.rs"]
mod toolcards_tests;
mod truncate;
#[cfg(test)]
#[path = "truncate_tests.rs"]
mod truncate_tests;

pub(crate) use assistant_segments::{
    assistant_segments_to_lines_with_meta, parse_assistant_segments,
};
#[cfg(test)]
pub(crate) use structured_stream::build_streaming_transcript_view;
#[cfg(test)]
pub(crate) use summary::transcript_to_lines;
pub(crate) use toolcards::{render_tool_card, tool_summary_cli};
pub(crate) use truncate::truncate_chars_cli;

use super::theme::{ACCENT_CYAN, ACCENT_RED, SUCCESS, TEXT_SUBTLE, WARNING};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::collections::BTreeSet;

/// 思考块的交互元数据
///
/// `id` 用于在渲染层与 TUI 交互层之间关联同一个思考块，
/// `open` 表示该块是否仍处于流式思考中（尚未遇到闭合标签）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ThinkBlockMeta {
    pub(crate) id: u64,
    pub(crate) open: bool,
}

/// 计算思考块当前是否应当展开
///
/// 规则如下：
/// - 全局展开开启时，所有思考块都展开。
/// - 否则按块的默认状态决定：`open=true` 默认展开，`open=false` 默认折叠。
/// - 若该块存在于覆盖集合中，则表示用户手动切换过一次，应反转默认状态。
pub(crate) fn think_block_expanded(
    meta: ThinkBlockMeta,
    expand_think_all: bool,
    think_detail_overrides: &BTreeSet<u64>,
) -> bool {
    if expand_think_all {
        return true;
    }

    if think_detail_overrides.contains(&meta.id) { !meta.open } else { meta.open }
}

#[cfg(test)]
mod tests;

/// 对话条目的角色类型
///
/// 定义对话中每个条目的发送者或类型，用于决定显示样式。
/// 不同角色在 CLI 界面中使用不同的颜色和格式进行区分。
#[derive(Clone, Copy)]
pub(crate) enum TranscriptRole {
    /// 系统消息（如提示词、内部状态）
    System,
    /// 用户输入的消息
    User,
    /// 助手（AI）生成的回复
    Assistant,
    /// 进度指示信息（如正在执行的任务）
    Progress,
    /// 错误消息（如执行失败、异常）
    Error,
}

/// 单条对话记录条目
///
/// 表示对话中的一条记录，包含角色、时间戳和文本内容。
/// 这是对话记录的基本存储单元，用于在 CLI 界面中展示历史对话。
#[derive(Clone)]
pub(crate) struct TranscriptEntry {
    /// 消息的角色类型
    pub(crate) role: TranscriptRole,
    /// 消息创建时间（本地时区）
    pub(crate) at: chrono::DateTime<chrono::Local>,
    /// 消息文本内容
    pub(crate) text: String,
}

impl TranscriptEntry {
    /// 创建新的对话条目
    ///
    /// 自动将时间戳设置为当前本地时间。
    ///
    /// # 参数
    ///
    /// - `role`: 消息角色类型
    /// - `text`: 消息文本内容（支持任何实现 `Into<String>` 的类型）
    ///
    /// # 返回值
    ///
    /// 返回一个新的 `TranscriptEntry` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let user_msg = TranscriptEntry::new(TranscriptRole::User, "你好");
    /// let assistant_msg = TranscriptEntry::new(TranscriptRole::Assistant, String::from("你好！"));
    /// ```
    pub(crate) fn new(role: TranscriptRole, text: impl Into<String>) -> Self {
        Self { role, at: chrono::Local::now(), text: text.into() }
    }
}

/// 返回禁用修剪的文本换行配置
///
/// 用于 ratatui 组件中配置文本换行行为，设置为不自动修剪行尾空白。
/// 这在显示需要保留原始格式的文本时很有用。
///
/// # 返回值
///
/// 返回 `Wrap { trim: false }` 配置
///
/// # 示例
///
/// ```ignore
/// use ratatui::widgets::Paragraph;
///
/// let paragraph = Paragraph::new("text")
///     .wrap(wrap_trim_disabled());
/// ```
pub(crate) fn wrap_trim_disabled() -> ratatui::widgets::Wrap {
    ratatui::widgets::Wrap { trim: false }
}

/// 获取对话角色的前缀样式配置
///
/// 根据角色类型返回对应的前缀字符串、颜色和样式。
/// 这些样式用于在 CLI 界面中区分不同来源的消息。
///
/// # 参数
///
/// - `role`: 对话角色类型
///
/// # 返回值
///
/// 返回元组 `(prefix, color, style)`：
/// - `prefix`: 前缀字符串（当前实现为空字符串）
/// - `color`: 前景颜色
/// - `style`: 完整的样式（包含颜色和粗体修饰）
///
/// # 样式映射
///
/// | 角色 | 颜色 |
/// |------|------|
/// | System | DarkGray |
/// | User | Green |
/// | Assistant | Cyan |
/// | Progress | Yellow |
/// | Error | Red |
///
/// # 示例
///
/// ```ignore
/// let (prefix, color, style) = transcript_prefix_style(TranscriptRole::Error);
/// // prefix = "", color = Color::Red, style = bold red
/// ```
pub(crate) fn transcript_prefix_style(role: TranscriptRole) -> (String, Color, Style) {
    let (prefix, color) = match role {
        TranscriptRole::System => ("", TEXT_SUBTLE),
        TranscriptRole::User => ("> ", SUCCESS),
        TranscriptRole::Assistant => ("● ", ACCENT_CYAN),
        TranscriptRole::Progress => ("", WARNING),
        TranscriptRole::Error => ("", ACCENT_RED),
    };
    let style = Style::default().fg(color).add_modifier(Modifier::BOLD);
    (prefix.to_string(), color, style)
}

/// 生成默认的空对话提示行
///
/// 当对话记录为空时，显示此提示信息引导用户开始使用。
/// 提示用户可以输入 `/help` 查看可用命令。
///
/// # 返回值
///
/// 返回包含提示文本的静态 `Line`，使用深灰色显示
///
/// # 示例
///
/// ```ignore
/// if transcript.is_empty() {
///     lines.push(default_empty_transcript_line());
/// }
/// ```
pub(crate) fn default_empty_transcript_line() -> Line<'static> {
    Line::from(Span::styled("Ready. Type /help for commands.", Style::default().fg(TEXT_SUBTLE)))
}
