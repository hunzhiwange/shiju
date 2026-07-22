//! CLI Logo 模块
//!
//! 本模块提供 VibeWindow 的 ASCII 艺术字 logo 生成功能，用于在终端界面中
//! 展示品牌标识。生成的 logo 采用 ratatui 库的 `Line` 和 `Span` 类型，
//! 支持自定义颜色和高亮样式，可直接集成到 TUI 界面中。
//!
//! # 功能特性
//!
//! - 生成 "VibeWindow" 的 ASCII 艺术字 logo
//! - 支持高亮样式（首尾字母 V 和 Window 中的 W 采用红色高亮）
//! - 其余字母采用灰色暗淡样式，形成视觉层次
//! - 返回 4 行高度的完整 logo，适合在终端顶部展示
//!
//! # 示例
//!
//! ```ignore
//! use crate::app::agent::agent::loop_::cli::logo::logo_text_lines;
//!
//! let logo_lines = logo_text_lines();
//! // logo_lines 包含 4 行 Line<'static>，可直接用于 ratatui 渲染
//! ```

use super::theme::{ACCENT_CYAN, ACCENT_RED, TEXT_MUTED};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

/// 生成 VibeWindow ASCII 艺术字 logo 的文本行
///
/// 该函数创建 "VibeWindow" 字符串的 ASCII 艺术字表示，
/// 采用 4 行高度的字符画形式。logo 中 "V" 和 "Window" 部分
/// 的首字母 "W" 采用红色高亮，其余字符采用灰色暗淡样式，
/// 形成品牌识别度。
///
/// # 返回值
///
/// 返回包含 4 个 `Line<'static>` 的向量，每个 `Line` 代表 logo 的一行。
/// 返回的行可直接传递给 ratatui 的渲染管道进行显示。
///
/// # 样式说明
///
/// - **高亮字符**: "V" 和 "Window" 中的 "W" 使用红色前景色
/// - **基础字符**: 其余字符使用灰色前景色并添加暗淡修饰符
/// - **字符间距**: 每个字母之间使用空格分隔，提升可读性
///
/// # 示例
///
/// ```ignore
/// let lines = logo_text_lines();
/// assert_eq!(lines.len(), 4);
///
/// // 在 ratatui 中渲染
/// let area = Rect::new(0, 0, 50, 4);
/// for (i, line) in lines.iter().enumerate() {
///     frame.render_widget(line.clone(), Rect::new(area.x, area.y + i as u16, area.width, 1));
/// }
/// ```
///
/// # 实现细节
///
/// 函数内部定义了每个字母的 4 行 ASCII 字符数组：
/// - `v`: 字母 V 的 4x5 字符矩阵
/// - `w`: 字母 W 的 4x9 字符矩阵
/// - `i`: 字母 I 的 4x4 字符矩阵
/// - `b`: 字母 B 的 4x4 字符矩阵
/// - `e`: 字母 E 的 4x4 字符矩阵
/// - `n`: 字母 N 的 4x4 字符矩阵
/// - `d`: 字母 D 的 4x4 字符矩阵
/// - `o`: 字母 O 的 4x4 字符矩阵
///
/// 通过逐行组装这些字符块，形成完整的 "VibeWindow" logo。
pub(crate) fn logo_text_lines(_scale: usize) -> Vec<Line<'static>> {
    let red_bold = Style::default().fg(ACCENT_RED).add_modifier(Modifier::BOLD);
    let cyan_bold = Style::default().fg(ACCENT_CYAN).add_modifier(Modifier::BOLD);
    let en_style = Style::default().fg(TEXT_MUTED);

    vec![
        Line::from(vec![
            Span::styled("氛", red_bold),
            Span::styled("围", cyan_bold),
            Span::styled("视", red_bold),
            Span::styled("窗", cyan_bold),
        ]),
        Line::from(vec![Span::styled("VibeWindow", en_style)]),
    ]
}
