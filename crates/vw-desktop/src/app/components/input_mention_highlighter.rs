//! 输入框提及（@mention）高亮器模块
//!
//! 本模块提供了用于在文本输入框中识别和高亮显示 `@mention` 的功能。
//! 提及是指以 `@` 符号开头的文本片段，通常用于引用用户、频道或其他实体。
//!
//! # 主要组件
//!
//! - [`MentionHighlighter`]: 核心高亮器实现，负责扫描文本并识别提及
//! - [`Highlight`]: 高亮信息结构，标记文本是否为提及
//! - [`Settings`]: 高亮器配置（当前为空实现，预留扩展）
//!
//! # 功能特性
//!
//! - 自动识别 `@` 符号开头的提及文本
//! - 支持多种字符类型的提及（字母、数字、路径分隔符等）
//! - 提供两种格式化选项：正常显示和隐藏
//! - 实现了 iced 框架的 `Highlighter` trait，可无缝集成到 iced 应用中
//!
//! # 示例
//!
//! ```ignore
//! use input_mention_highlighter::{MentionHighlighter, mention_format};
//!
//! // 创建高亮器
//! let highlighter = MentionHighlighter::new(&Settings);
//!
//! // 高亮一行文本
//! let line = "Hello @user/world, check @project#123";
//! let highlights = highlighter.highlight_line(line);
//! ```

use iced::advanced::text::highlighter;
use std::ops::Range;

/// 提及高亮器的配置设置
///
/// 当前为空结构体，预留未来扩展配置项使用。
/// 可以在未来添加诸如提及颜色、是否启用等配置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings;

/// 高亮信息
///
/// 用于标记文本片段是否为提及（@mention）。
/// 这个结构体会被高亮器返回，用于指示文本的格式化方式。
#[derive(Debug, Clone, Copy)]
pub struct Highlight {
    /// 标记该文本片段是否为提及
    pub is_mention: bool,
}

/// 提及高亮器
///
/// 实现了 iced 框架的 `Highlighter` trait，用于扫描文本并识别 `@mention`。
///
/// # 工作原理
///
/// 高亮器会逐行扫描文本，查找以 `@` 符号开头的提及。识别到 `@` 后，
/// 会继续读取后续的有效字符，直到遇到无效字符为止。
///
/// # 支持的提及字符
///
/// 提及可以包含以下字符：
/// - Unicode 字母和数字（包括中文）
/// - 路径分隔符：`/` 和 `\`
/// - 点号：`.`
/// - 下划线：`_`
/// - 连字符：`-`
/// - 冒号：`:`
/// - 井号：`#`
///
/// # 示例
///
/// - `@user` - 简单用户提及
/// - `@user/workspace` - 带路径的用户提及
/// - `@project#123` - 带编号的项目提及
pub struct MentionHighlighter {
    /// 当前处理的行号（从0开始）
    current_line: usize,
}

/// Unicode 字母数字与路径、位置后缀字符。
pub(crate) fn is_mention_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '/' | '\\' | '.' | '_' | '-' | ':' | '#')
}

/// 返回提及的 UTF-8 字节范围；带空格或特殊字符的路径使用 JSON 字符串引号。
pub(crate) fn mention_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((start, c)) = chars.next() {
        if c != '@' {
            continue;
        }
        let mut end = start + 1;
        if chars.peek().is_some_and(|(_, c)| *c == '"') {
            chars.next();
            let mut escaped = false;
            for (index, c) in chars.by_ref() {
                if c == '\n' || c == '\r' {
                    break;
                }
                if c == '"' && !escaped {
                    let candidate = &text[start + 1..index + 1];
                    if serde_json::from_str::<String>(candidate).is_ok_and(|path| !path.is_empty())
                    {
                        end = index + 1;
                    }
                    break;
                }
                escaped = c == '\\' && !escaped;
            }
        } else {
            while let Some(&(index, c)) = chars.peek() {
                if !is_mention_char(c) {
                    break;
                }
                end = index + c.len_utf8();
                chars.next();
            }
        }
        if end > start + 1 {
            ranges.push(start..end);
        }
    }
    ranges
}

/// 将文件路径编码为可完整提取和删除的提及。
pub(crate) fn file_mention(path: &str) -> String {
    if path.chars().all(is_mention_char) {
        format!("@{path}")
    } else {
        format!("@{}", serde_json::to_string(path).expect("file path is serializable"))
    }
}

impl iced::advanced::text::highlighter::Highlighter for MentionHighlighter {
    /// 高亮器配置类型
    type Settings = Settings;
    /// 高亮信息类型
    type Highlight = Highlight;
    /// 高亮结果迭代器类型，返回字节范围和高亮信息的配对
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Self::Highlight)>;

    /// 创建新的高亮器实例
    ///
    /// # 参数
    ///
    /// - `_settings`: 高亮器配置（当前未使用）
    ///
    /// # 返回值
    ///
    /// 返回初始化的高亮器实例，行号从 0 开始
    fn new(_settings: &Self::Settings) -> Self {
        Self { current_line: 0 }
    }

    /// 更新高亮器配置
    ///
    /// 当配置变更时调用，会重置行号计数器。
    ///
    /// # 参数
    ///
    /// - `_new_settings`: 新的配置（当前未使用）
    fn update(&mut self, _new_settings: &Self::Settings) {
        self.current_line = 0;
    }

    /// 设置当前处理的行号
    ///
    /// 用于在跳转或重新定位时更新行号。
    ///
    /// # 参数
    ///
    /// - `line`: 新的行号
    fn change_line(&mut self, line: usize) {
        self.current_line = line;
    }

    /// 高亮单行文本
    ///
    /// 扫描给定行的文本，识别所有 `@mention` 并返回它们的位置和高亮信息。
    ///
    /// # 算法说明
    ///
    /// 1. 按 Unicode 字符扫描文本
    /// 2. 遇到 `@` 符号时，开始收集后续的有效提及字符
    /// 3. 收集到至少一个有效字符后，标记为提及并记录范围
    /// 4. 继续扫描直到行尾
    ///
    /// # 参数
    ///
    /// - `line`: 要高亮的文本行
    ///
    /// # 返回值
    ///
    /// 返回迭代器，包含所有识别到的提及的字节范围和对应的高亮信息
    ///
    /// # 示例
    ///
    /// 输入: `"Hello @user, meet @admin"`
    /// 输出: `[(6..11, Highlight { is_mention: true }), (18..24, Highlight { is_mention: true })]`
    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let ranges = mention_ranges(line)
            .into_iter()
            .map(|range| (range, Highlight { is_mention: true }))
            .collect::<Vec<_>>();

        // 更新行号（使用 saturating_add 防止溢出）
        self.current_line = self.current_line.saturating_add(1);
        ranges.into_iter()
    }

    /// 获取当前处理的行号
    ///
    /// # 返回值
    ///
    /// 返回当前行号
    fn current_line(&self) -> usize {
        self.current_line
    }
}

/// 提及的正常格式化函数
///
/// 将提及文本格式化为主题色的颜色，使其在视觉上突出显示。
///
/// # 参数
///
/// - `highlight`: 高亮信息，用于判断是否为提及
/// - `theme`: iced 主题，用于获取颜色配置
///
/// # 返回值
///
/// 返回格式化信息，包含颜色和字体设置。如果是提及，使用主题的主色调；
/// 否则使用默认格式。
///
/// # 示例
///
/// ```ignore
/// let format = mention_format(&Highlight { is_mention: true }, &theme);
/// // format.color 将是主题的主色调
/// ```
pub fn mention_format(
    highlight: &Highlight,
    theme: &iced::Theme,
) -> highlighter::Format<iced::Font> {
    if highlight.is_mention {
        highlighter::Format { color: Some(theme.palette().primary), font: None }
    } else {
        highlighter::Format::default()
    }
}

/// 提及的隐藏格式化函数
///
/// 将提及文本格式化为透明色，使其在视觉上隐藏。
/// 这在某些场景下很有用，例如当提及需要特殊渲染时。
///
/// # 参数
///
/// - `highlight`: 高亮信息，用于判断是否为提及
/// - `_theme`: iced 主题（未使用，保留以保持函数签名一致）
///
/// # 返回值
///
/// 返回格式化信息。如果是提及，颜色设置为透明；
/// 否则使用默认格式。
///
/// # 使用场景
///
/// 当需要将提及渲染为自定义组件（如标签、按钮等）时，
/// 可以先用此函数隐藏原始文本，然后在上面绘制自定义组件。
pub fn mention_hidden_format(
    highlight: &Highlight,
    _theme: &iced::Theme,
) -> highlighter::Format<iced::Font> {
    if highlight.is_mention {
        highlighter::Format { color: Some(iced::Color::TRANSPARENT), font: None }
    } else {
        highlighter::Format::default()
    }
}

/// 提及格式化函数类型别名
///
/// 定义了提及格式化函数的签名，便于在代码中传递和使用。
pub type MentionFormatter = fn(&Highlight, &iced::Theme) -> highlighter::Format<iced::Font>;

/// 根据配置选择提及显示格式的工厂函数
///
/// 根据是否显示原始提及文本，返回相应的格式化函数。
///
/// # 参数
///
/// - `show_raw_mention`: 是否显示原始提及文本
///   - `true`: 使用 [`mention_format`]，以主题色显示提及
///   - `false`: 使用 [`mention_hidden_format`]，隐藏提及文本
///
/// # 返回值
///
/// 返回相应的格式化函数
///
/// # 示例
///
/// ```ignore
/// // 显示提及
/// let formatter = mention_display_format(true);
/// let format = formatter(&Highlight { is_mention: true }, &theme);
///
/// // 隐藏提及
/// let formatter = mention_display_format(false);
/// let format = formatter(&Highlight { is_mention: true }, &theme);
/// ```
pub fn mention_display_format(show_raw_mention: bool) -> MentionFormatter {
    if show_raw_mention { mention_format } else { mention_hidden_format }
}
