//! Git 面板工具模块
//!
//! 本模块提供 Git 差异视图所需的各种工具函数和类型定义，包括：
//! - 文件状态枚举（`FileStatus`）
//! - 编程语言枚举（`Lang`）及识别函数
//! - 语法高亮支持
//! - 单词级别差异计算
//! - 差异内容渲染
//! - 主题配色方案
//!
//! # 主要功能
//!
//! 1. **语言识别**：根据文件扩展名识别编程语言
//! 2. **语法高亮**：对代码行进行简单的关键字高亮
//! 3. **差异高亮**：在单词级别标记文本差异
//! 4. **UI 渲染**：将差异内容渲染为 Iced UI 组件
//! 5. **主题配色**：提供 GitHub 和 Monokai 两种配色方案

use std::ops::Range;

use iced::widget::{container, row, text};
use iced::{Background, Color};

use crate::app::{DiffTheme, Message};

/// Git 文件状态枚举
///
/// 表示 Git 仓库中文件的不同状态，用于在差异视图中区分文件变更类型。
///
/// # 变体
///
/// - `Modified` - 文件已修改
/// - `Added` - 文件新增
/// - `Deleted` - 文件已删除
/// - `Renamed` - 文件已重命名
/// - `Untracked` - 未跟踪文件
/// - `Unknown` - 未知状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Unknown,
}

/// 编程语言枚举
///
/// 表示支持的编程语言类型，用于语法高亮和语言特定处理。
///
/// # 支持的语言
///
/// - `Rust` - Rust 语言
/// - `Js` - JavaScript 语言
/// - `Ts` - TypeScript 语言
/// - `Json` - JSON 格式
/// - `Toml` - TOML 配置格式
/// - `Yaml` - YAML 配置格式
/// - `Python` - Python 语言
/// - `Go` - Go 语言
/// - `C` - C 语言
/// - `Cpp` - C++ 语言
/// - `Html` - HTML 标记语言
/// - `Css` - CSS 样式语言
/// - `Sql` - SQL 查询语言
/// - `Bash` - Bash 脚本语言
/// - `Other` - 其他/未知语言
#[derive(Clone, Copy)]
pub enum Lang {
    Rust,
    Js,
    Ts,
    Json,
    Toml,
    Yaml,
    Python,
    Go,
    C,
    Cpp,
    Html,
    Css,
    Sql,
    Bash,
    Other,
}

/// 根据文件名识别编程语言
///
/// 通过检查文件扩展名来判断文件所属的编程语言类型。
///
/// # 参数
///
/// * `name` - 文件名（包含扩展名）
///
/// # 返回值
///
/// 返回对应的 `Lang` 枚举值
///
/// # 示例
///
/// ```ignore
/// let lang = lang_for_file("main.rs");
/// assert_eq!(lang, Lang::Rust);
///
/// let lang = lang_for_file("config.json");
/// assert_eq!(lang, Lang::Json);
/// ```
pub fn lang_for_file(name: &str) -> Lang {
    if name.ends_with(".rs") {
        Lang::Rust
    } else if name.ends_with(".ts") || name.ends_with(".tsx") {
        Lang::Ts
    } else if name.ends_with(".js") || name.ends_with(".jsx") {
        Lang::Js
    } else if name.ends_with(".json") {
        Lang::Json
    } else if name.ends_with(".toml") {
        Lang::Toml
    } else if name.ends_with(".yaml") || name.ends_with(".yml") {
        Lang::Yaml
    } else if name.ends_with(".py") {
        Lang::Python
    } else if name.ends_with(".go") {
        Lang::Go
    } else if name.ends_with(".c") {
        Lang::C
    } else if name.ends_with(".cpp")
        || name.ends_with(".cc")
        || name.ends_with(".cxx")
        || name.ends_with(".hpp")
    {
        Lang::Cpp
    } else if name.ends_with(".html") || name.ends_with(".htm") {
        Lang::Html
    } else if name.ends_with(".css") {
        Lang::Css
    } else if name.ends_with(".sql") {
        Lang::Sql
    } else if name.ends_with(".sh") || name.ends_with(".bash") {
        Lang::Bash
    } else {
        Lang::Other
    }
}

/// 获取编程语言的默认文件扩展名
///
/// # 参数
///
/// * `lang` - 编程语言枚举值
///
/// # 返回值
///
/// 返回该语言的典型文件扩展名（不含点号）
#[allow(dead_code)]
fn lang_extension(lang: Lang) -> &'static str {
    match lang {
        Lang::Rust => "rs",
        Lang::Js => "js",
        Lang::Ts => "ts",
        Lang::Json => "json",
        Lang::Toml => "toml",
        Lang::Yaml => "yaml",
        Lang::Python => "py",
        Lang::Go => "go",
        Lang::C => "c",
        Lang::Cpp => "cpp",
        Lang::Html => "html",
        Lang::Css => "css",
        Lang::Sql => "sql",
        Lang::Bash => "sh",
        Lang::Other => "txt",
    }
}

/// 判断字符是否为单词字符
///
/// 单词字符包括 ASCII 字母、数字和下划线，用于语法高亮分段。
///
/// # 参数
///
/// * `c` - 待判断的字符
///
/// # 返回值
///
/// 如果是单词字符返回 `true`，否则返回 `false`
fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// 计算代码行的高亮片段
///
/// 将一行代码分割成多个片段，并标记哪些片段是关键字。
/// 片段按照单词字符和非单词字符边界进行划分。
///
/// # 参数
///
/// * `line` - 待分析的代码行
/// * `_lang` - 编程语言（当前未使用，预留用于未来扩展）
/// * `_theme` - 差异主题（当前未使用，预留用于未来扩展）
///
/// # 返回值
///
/// 返回一个元组向量，每个元组包含三个元素：
/// - `usize` - 片段起始位置（字节偏移）
/// - `usize` - 片段结束位置（字节偏移）
/// - `bool` - 是否为关键字
///
/// # 实现细节
///
/// 1. 遍历每个字符，根据 `is_word_char` 判断字符类型
/// 2. 当字符类型发生变化时，创建新的片段
/// 3. 检查单词片段是否匹配已知关键字列表
/// 4. 支持的关键字：`if`、`let`、`var`、`else`、`fn`、`function`、`class`、`export`
pub fn highlight_segments(line: &str, _lang: Lang, _theme: DiffTheme) -> Vec<(usize, usize, bool)> {
    let mut out: Vec<(usize, usize, bool)> = Vec::new();
    if line.is_empty() {
        return out;
    }

    let mut seg_start = 0usize;
    let mut prev_kind: Option<bool> = None;

    for (idx, ch) in line.char_indices() {
        let kind = is_word_char(ch);
        match prev_kind {
            None => prev_kind = Some(kind),
            Some(pk) if pk != kind => {
                let is_kw = if pk {
                    matches!(
                        &line[seg_start..idx],
                        "if" | "let" | "var" | "else" | "fn" | "function" | "class" | "export"
                    )
                } else {
                    false
                };
                if seg_start < idx {
                    out.push((seg_start, idx, is_kw));
                }
                seg_start = idx;
                prev_kind = Some(kind);
            }
            _ => {}
        }
    }

    let end = line.len();
    let pk = prev_kind.unwrap_or(false);
    let is_kw = if pk {
        matches!(
            &line[seg_start..end],
            "if" | "let" | "var" | "else" | "fn" | "function" | "class" | "export"
        )
    } else {
        false
    };
    if seg_start < end {
        out.push((seg_start, end, is_kw));
    }

    out
}

/// 计算单词级别的差异范围
///
/// 使用 `similar` 库对新旧文本进行单词级别的差异分析，
/// 返回在旧文本和新文本中发生变更的字符范围。
///
/// # 参数
///
/// * `old` - 旧文本内容
/// * `new` - 新文本内容
///
/// # 返回值
///
/// 返回一个元组，包含两个 `Range<usize>` 向量：
/// - 第一个向量：旧文本中被删除内容的字符范围
/// - 第二个向量：新文本中插入内容的字符范围
///
/// # 示例
///
/// ```ignore
/// let (old_ranges, new_ranges) = get_word_diff_ranges(
///     "hello world",
///     "hello rust"
/// );
/// // old_ranges 包含 "world" 的范围
/// // new_ranges 包含 "rust" 的范围
/// ```
pub fn get_word_diff_ranges(
    old: &str,
    new: &str,
) -> (Vec<std::ops::Range<usize>>, Vec<std::ops::Range<usize>>) {
    let diff = similar::TextDiff::from_words(old, new);
    let mut old_ranges = Vec::new();
    let mut new_ranges = Vec::new();
    let mut old_idx = 0;
    let mut new_idx = 0;

    for change in diff.iter_all_changes() {
        let len = change.value().len();
        match change.tag() {
            similar::ChangeTag::Delete => {
                if let Some(range) = trim_diff_range_whitespace(old, old_idx..old_idx + len) {
                    old_ranges.push(range);
                }
                old_idx += len;
            }
            similar::ChangeTag::Insert => {
                if let Some(range) = trim_diff_range_whitespace(new, new_idx..new_idx + len) {
                    new_ranges.push(range);
                }
                new_idx += len;
            }
            similar::ChangeTag::Equal => {
                old_idx += len;
                new_idx += len;
            }
        }
    }
    (old_ranges, new_ranges)
}

fn trim_diff_range_whitespace(content: &str, range: Range<usize>) -> Option<Range<usize>> {
    if range.start >= range.end || range.end > content.len() {
        return None;
    }

    let slice = &content[range.clone()];
    let trimmed = slice.trim_matches(char::is_whitespace);
    if trimmed.is_empty() {
        return Some(range);
    }

    let leading = slice.len().saturating_sub(slice.trim_start_matches(char::is_whitespace).len());
    let trailing = slice.len().saturating_sub(slice.trim_end_matches(char::is_whitespace).len());
    let start = range.start + leading;
    let end = range.end.saturating_sub(trailing);

    (start < end).then_some(start..end)
}

/// 渲染差异行内容
///
/// 将差异内容渲染为 Iced UI 组件，支持语法高亮和差异高亮。
///
/// # 参数
///
/// * `content` - 要渲染的文本内容
/// * `lang` - 编程语言，用于语法高亮
/// * `theme` - 差异主题配色方案
/// * `highlight_syntax` - 是否启用语法高亮
/// * `ranges` - 需要高亮显示的字符范围（通常表示差异部分）
/// * `base_bg` - 基础背景色
/// * `highlight_bg` - 高亮背景色
///
/// # 返回值
///
/// 返回一个 Iced `Row` 组件，包含渲染后的文本片段
///
/// # 实现细节
///
/// 1. 如果启用语法高亮，调用 `highlight_segments` 获取片段和关键字标记
/// 2. 关键字使用特定颜色（红棕色 `#D73A49`）
/// 3. 遍历每个片段，根据 `ranges` 判断是否在差异范围内
/// 4. 差异范围内的文本使用 `highlight_bg` 背景色
/// 5. 差异范围外的文本使用 `base_bg` 背景色
/// 6. 每个文本片段包装在 `container` 中，带有适当的样式
#[derive(Clone, Copy)]
pub(super) struct LineContentPart {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) color: Option<Color>,
    pub(super) is_highlighted: bool,
}

pub(super) fn line_content_parts(
    content: &str,
    lang: Lang,
    theme: DiffTheme,
    highlight_syntax: bool,
    ranges: &[Range<usize>],
) -> Vec<LineContentPart> {
    let keyword_color = Color::from_rgb8(0xD7, 0x3A, 0x49);
    let segments: Vec<(usize, usize, Option<Color>)> = if highlight_syntax {
        highlight_segments(content, lang, theme)
            .into_iter()
            .map(|(s, e, is_kw)| (s, e, if is_kw { Some(keyword_color) } else { None }))
            .collect()
    } else {
        vec![(0, content.len(), None)]
    };
    let mut parts = Vec::with_capacity(segments.len().saturating_add(ranges.len()));

    if ranges.is_empty() {
        for (seg_start, seg_end, color) in segments {
            if seg_start < seg_end {
                parts.push(LineContentPart {
                    start: seg_start,
                    end: seg_end,
                    color,
                    is_highlighted: false,
                });
            }
        }
        return parts;
    }

    let mut range_idx = 0;

    for (seg_start, seg_end, color) in segments {
        if seg_start >= seg_end {
            continue;
        }

        while range_idx < ranges.len() && ranges[range_idx].end <= seg_start {
            range_idx += 1;
        }

        let mut current_pos = seg_start;
        let mut current_range_idx = range_idx;

        while current_pos < seg_end {
            while current_range_idx < ranges.len() && ranges[current_range_idx].end <= current_pos {
                current_range_idx += 1;
            }

            let next_range = ranges.get(current_range_idx).filter(|range| range.start < seg_end);
            let (limit, is_highlighted) = if let Some(range) = next_range {
                if range.start <= current_pos {
                    (range.end.min(seg_end), true)
                } else {
                    (range.start.min(seg_end), false)
                }
            } else {
                (seg_end, false)
            };

            if limit <= current_pos {
                break;
            }

            parts.push(LineContentPart { start: current_pos, end: limit, color, is_highlighted });

            current_pos = limit;
        }

        range_idx = current_range_idx;
    }

    parts
}

fn build_line_content_row(
    content: &str,
    parts: &[LineContentPart],
    base_bg: Color,
    highlight_bg: Color,
) -> iced::widget::Row<'static, Message> {
    let mut row = row![];

    for part in parts {
        let chunk_text = &content[part.start..part.end];
        let bg = if part.is_highlighted { highlight_bg } else { base_bg };
        row = row.push(
            container(
                if let Some(color) = part.color {
                    text(chunk_text.to_owned()).color(color)
                } else {
                    text(chunk_text.to_owned())
                }
                .size(13)
                .line_height(iced::widget::text::LineHeight::Relative(1.2)),
            )
            .padding([2, 0])
            .style(move |_| iced::widget::container::Style {
                text_color: None,
                background: Some(Background::Color(bg)),
                border: iced::Border::default(),
                shadow: iced::Shadow::default(),
                snap: false,
            }),
        );
    }

    row
}

pub fn render_line_content(
    content: &str,
    lang: Lang,
    theme: DiffTheme,
    highlight_syntax: bool,
    ranges: &[Range<usize>],
    base_bg: Color,
    highlight_bg: Color,
) -> iced::widget::Row<'static, Message> {
    let parts = line_content_parts(content, lang, theme, highlight_syntax, ranges);
    build_line_content_row(content, &parts, base_bg, highlight_bg)
}

/// 获取差异视图的配色方案
///
/// 根据指定的主题返回差异视图所需的 8 种颜色。
///
/// # 参数
///
/// * `theme` - 差异主题枚举值
///
/// # 返回值
///
/// 返回一个包含 8 个 `Color` 的元组，顺序为：
/// 1. 普通行背景色
/// 2. 普通行文本色
/// 3. 新增行背景色（淡色）
/// 4. 新增行背景色（深色/高亮）
/// 5. 删除行背景色（淡色）
/// 6. 删除行背景色（深色/高亮）
/// 7. 修改行背景色（淡色）
/// 8. 修改行文本色/高亮色
///
/// # 主题说明
///
/// ## GitHub 主题
/// - 采用 GitHub PR 界面的配色风格
/// - 绿色系表示新增，红色系表示删除
/// - 整体色调偏冷、柔和
///
/// ## Monokai 主题
/// - 采用经典 Monokai 配色方案
/// - 深色背景，高对比度
/// - 适合暗色主题环境
pub fn get_diff_colors(
    theme: DiffTheme,
) -> (Color, Color, Color, Color, Color, Color, Color, Color) {
    match theme {
        DiffTheme::GitHub => (
            Color::TRANSPARENT,
            Color::from_rgb8(0x24, 0x29, 0x2E),
            Color::from_rgb8(0xEA, 0xF7, 0xEE),
            Color::from_rgb8(0xC7, 0xE9, 0xD1),
            Color::from_rgb8(0xFE, 0xEF, 0xEE),
            Color::from_rgb8(0xF7, 0xC9, 0xC5),
            Color::from_rgb8(0xEE, 0xF3, 0xF8),
            Color::from_rgb8(0x57, 0x60, 0x6A),
        ),
        DiffTheme::Monokai => (
            Color::TRANSPARENT,
            Color::from_rgb8(0xE6, 0xED, 0xF3),
            Color::from_rgb8(0x13, 0x2A, 0x1B),
            Color::from_rgb8(0x1E, 0x4B, 0x2C),
            Color::from_rgb8(0x31, 0x1A, 0x18),
            Color::from_rgb8(0x60, 0x30, 0x2B),
            Color::from_rgb8(0x1B, 0x20, 0x27),
            Color::from_rgb8(0xC9, 0xD1, 0xD9),
        ),
    }
}
