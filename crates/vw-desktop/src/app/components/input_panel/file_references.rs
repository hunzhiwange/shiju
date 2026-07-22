//! 文件引用组件模块
//!
//! 本模块提供输入面板中文件引用的解析、提取和渲染功能。
//! 用户可以通过 `@文件路径` 的方式在消息中引用文件，支持多种位置格式：
//!
//! # 支持的引用格式
//!
//! - `@path/to/file.rs` - 仅引用文件
//! - `@path/to/file.rs:10` - 引用特定行
//! - `@path/to/file.rs:10:5` - 引用特定行列位置
//! - `@path/to/file.rs:10-20` - 引用行范围
//! - `@path/to/file.rs:10:5-20:8` - 引用精确范围（行列起止）
//!
//! # 主要组件
//!
//! - [`FileReferenceLocation`] - 文件位置的枚举类型
//! - [`extract_file_mentions`] - 从文本中提取文件引用
//! - [`render_file_references`] - 渲染文件引用卡片列表

use crate::app::{Message, message};
use iced::widget::tooltip::Position;
use iced::widget::{button, container, mouse_area, row, text, tooltip};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::app::assets::{self, Icon};

/// 文件引用位置类型
///
/// 表示文件引用中的不同位置精度级别，从单行到精确的行列范围。
#[derive(Debug, Clone, Copy)]
pub(super) enum FileReferenceLocation {
    /// 单行位置，如 `file.rs:10`
    Line { line: usize },
    /// 行列位置，如 `file.rs:10:5`
    LineColumn { line: usize, column: usize },
    /// 行范围，如 `file.rs:10-20`
    LineRange { start_line: usize, end_line: usize },
    /// 精确范围（含行列），如 `file.rs:10:5-20:8`
    Range { start_line: usize, start_column: usize, end_line: usize, end_column: usize },
}

/// 解析后的文件引用结构
///
/// 包含文件路径和可选的位置信息。
#[derive(Debug, Clone)]
pub(super) struct ParsedFileReference {
    /// 文件路径（不含位置后缀）
    pub(super) path: String,
    /// 可选的位置信息
    pub(super) location: Option<FileReferenceLocation>,
}

/// 解析文件引用字符串
///
/// 支持多种格式的文件引用解析，按照从最精确到最简单的顺序尝试匹配：
/// 1. 完整范围 `path:start_line:start_col-end_line:end_col`
/// 2. 行列位置 `path:line:column`
/// 3. 行范围 `path:start_line-end_line`
/// 4. 单行 `path:line`
/// 5. 仅路径 `path`
///
/// # 参数
///
/// - `reference` - 原始文件引用字符串（不含 @ 符号）
///
/// # 返回值
///
/// 返回解析后的 [`ParsedFileReference`]，即使解析失败也会返回路径部分
pub(super) fn parse_file_reference(reference: &str) -> ParsedFileReference {
    /// 解析数字字符串为 usize
    ///
    /// 空字符串返回 None，否则尝试解析
    fn parse_num(s: &str) -> Option<usize> {
        (!s.is_empty()).then_some(())?;
        s.parse::<usize>().ok()
    }

    // 尝试解析完整范围格式：path:start_line:start_col-end_line:end_col
    // 这是最精确的格式，需要同时匹配起止行列
    if let Some((before_end_col, end_column_str)) = reference.rsplit_once(':')
        && let Some(end_column) = parse_num(end_column_str)
        && let Some((before_end_line, end_line_str)) = before_end_col.rsplit_once('-')
        && let Some(end_line) = parse_num(end_line_str)
        && let Some((before_start_col, start_column_str)) = before_end_line.rsplit_once(':')
        && let Some(start_column) = parse_num(start_column_str)
        && let Some((path, start_line_str)) = before_start_col.rsplit_once(':')
        && let Some(start_line) = parse_num(start_line_str)
        && !path.is_empty()
    {
        return ParsedFileReference {
            path: path.to_string(),
            location: Some(FileReferenceLocation::Range {
                start_line,
                start_column,
                end_line,
                end_column,
            }),
        };
    }

    // 尝试解析行列格式：path:line:column
    if let Some((before_column, column_str)) = reference.rsplit_once(':')
        && let Some(column) = parse_num(column_str)
        && let Some((path, line_str)) = before_column.rsplit_once(':')
        && let Some(line) = parse_num(line_str)
        && !path.is_empty()
    {
        return ParsedFileReference {
            path: path.to_string(),
            location: Some(FileReferenceLocation::LineColumn { line, column }),
        };
    }

    // 尝试解析行范围格式：path:start_line-end_line
    if let Some((before_end_line, end_line_str)) = reference.rsplit_once('-')
        && let Some(end_line) = parse_num(end_line_str)
        && let Some((path, start_line_str)) = before_end_line.rsplit_once(':')
        && let Some(start_line) = parse_num(start_line_str)
        && !path.is_empty()
    {
        return ParsedFileReference {
            path: path.to_string(),
            location: Some(FileReferenceLocation::LineRange { start_line, end_line }),
        };
    }

    // 尝试解析单行格式：path:line
    if let Some((path, line_str)) = reference.rsplit_once(':')
        && let Some(line) = parse_num(line_str)
        && !path.is_empty()
    {
        return ParsedFileReference {
            path: path.to_string(),
            location: Some(FileReferenceLocation::Line { line }),
        };
    }

    // 无法解析位置信息，仅返回路径
    ParsedFileReference { path: reference.to_string(), location: None }
}

/// 格式化位置信息为人类可读的中文描述
///
/// # 参数
///
/// - `location` - 文件位置信息
///
/// # 返回值
///
/// 返回格式化的中文位置描述字符串
pub(super) fn format_reference_location(location: FileReferenceLocation) -> String {
    match location {
        FileReferenceLocation::Line { line } => format!("第 {} 行", line),
        FileReferenceLocation::LineColumn { line, column } => {
            format!("第 {} 行, 第 {} 列", line, column)
        }
        FileReferenceLocation::LineRange { start_line, end_line } => {
            format!("第 {}-{} 行", start_line, end_line)
        }
        FileReferenceLocation::Range { start_line, start_column, end_line, end_column } => {
            format!(
                "第 {} 行, 第 {} 列 到 第 {} 行, 第 {} 列",
                start_line, start_column, end_line, end_column
            )
        }
    }
}

/// 格式化位置信息为紧凑格式（用于卡片显示）
///
/// 与 [`format_reference_location`] 不同，此函数输出简短格式，
/// 适合在有限的卡片空间内显示。
///
/// # 参数
///
/// - `location` - 文件位置信息
///
/// # 返回值
///
/// 返回紧凑格式的位置字符串
pub(super) fn format_reference_location_compact(location: FileReferenceLocation) -> String {
    match location {
        FileReferenceLocation::Line { line } => line.to_string(),
        FileReferenceLocation::LineColumn { line, column } => format!("{}:{}", line, column),
        FileReferenceLocation::LineRange { start_line, end_line } => {
            format!("{}-{}", start_line, end_line)
        }
        FileReferenceLocation::Range { start_line, start_column, end_line, end_column } => {
            format!("{}:{}-{}:{}", start_line, start_column, end_line, end_column)
        }
    }
}

/// 工具提示深色主题样式
///
/// 为悬停提示框提供深色背景、白色文字和圆角边框样式。
///
/// # 参数
///
/// - `_theme` - 当前主题（未使用，保持签名一致性）
///
/// # 返回值
///
/// 返回配置好的容器样式
pub(super) fn tooltip_dark_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        // 深色半透明背景
        background: Some(Background::Color(Color::from_rgba8(24, 24, 24, 0.96))),
        text_color: Some(Color::WHITE),
        // 圆角无边框
        border: Border { radius: 8.0.into(), width: 0.0, color: Color::TRANSPARENT },
        // 添加阴影效果增加层次感
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.40),
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}

/// 文件引用卡片样式
///
/// 根据悬停状态动态调整背景透明度和边框颜色。
/// 悬停时显示主题色边框，增强交互反馈。
///
/// # 参数
///
/// - `theme` - 当前主题
/// - `is_hovered` - 是否处于悬停状态
///
/// # 返回值
///
/// 返回配置好的容器样式
pub(super) fn file_reference_style(
    theme: &Theme,
    is_hovered: bool,
) -> iced::widget::container::Style {
    let p = theme.extended_palette();
    // 悬停时背景更不透明
    let bg = if is_hovered {
        p.background.weak.color.scale_alpha(0.9)
    } else {
        p.background.weak.color.scale_alpha(0.6)
    };
    // 悬停时显示主题色边框
    let border_color = if is_hovered {
        theme.palette().primary.scale_alpha(0.45)
    } else {
        p.background.strong.color.scale_alpha(0.9)
    };
    iced::widget::container::Style {
        background: Some(Background::Color(bg)),
        border: Border { radius: 8.0.into(), width: 1.0, color: border_color },
        text_color: Some(theme.palette().text),
        ..Default::default()
    }
}

/// 创建单个文件引用卡片组件
///
/// 卡片显示文件名和位置信息，悬停时：
/// - 显示删除按钮替代文件图标
/// - 边框变为主题色
/// - 显示完整路径的工具提示
///
/// # 参数
///
/// - `file_path` - 完整的文件引用字符串（含位置后缀）
/// - `is_hovered` - 是否处于悬停状态
///
/// # 返回值
///
/// 返回渲染好的 Element 组件
pub(super) fn file_reference_card<'a>(file_path: String, is_hovered: bool) -> Element<'a, Message> {
    let icon = Icon::FileText;
    let file_path_for_display = file_path.clone();
    let file_path_for_delete = file_path;
    let parsed_ref = parse_file_reference(&file_path_for_display);

    // 创建文件图标 SVG
    let icon_svg = iced::widget::svg::Svg::<iced::Theme>::new(assets::get_icon(icon))
        .width(Length::Fixed(14.0))
        .height(Length::Fixed(14.0))
        .style(move |theme: &Theme, _| iced::widget::svg::Style {
            color: Some(theme.palette().primary),
        });

    // 从完整路径中提取文件名用于显示
    let file_name = std::path::Path::new(&parsed_ref.path)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| parsed_ref.path.clone());

    // 前导图标：悬停时显示删除按钮，否则显示文件图标
    let leading_icon: Element<'_, Message> = if is_hovered {
        button(
            container(
                iced::widget::svg::Svg::<iced::Theme>::new(assets::get_icon(Icon::X))
                    .width(Length::Fixed(14.0))
                    .height(Length::Fixed(14.0))
                    .style(|theme: &Theme, _| iced::widget::svg::Style {
                        color: Some(theme.palette().text.scale_alpha(0.72)),
                    }),
            )
            .width(Length::Fixed(14.0))
            .height(Length::Fixed(14.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
        )
        .padding(0)
        .style(|theme: &Theme, status| {
            let p = theme.extended_palette();
            // 按钮悬停时显示背景
            let bg = match status {
                iced::widget::button::Status::Hovered => {
                    Some(Background::Color(p.background.strong.color.scale_alpha(0.5)))
                }
                _ => None,
            };
            iced::widget::button::Style {
                background: bg,
                border: Border { radius: 4.0.into(), width: 0.0, color: Color::TRANSPARENT },
                text_color: theme.palette().text,
                ..Default::default()
            }
        })
        .on_press(Message::Chat(message::ChatMessage::RemoveFileReference(
            file_path_for_delete.clone(),
        )))
        .into()
    } else {
        icon_svg.into()
    };

    // 获取紧凑格式的位置文本
    let location_text = parsed_ref.location.map(format_reference_location_compact);

    // 构建标题行：文件名 + 可选位置信息
    let title = if let Some(location) = location_text {
        row![
            text(file_name).size(12),
            text(location).size(11).style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.extended_palette().secondary.base.text),
            })
        ]
        .spacing(6)
        .align_y(Alignment::Center)
    } else {
        row![text(file_name).size(12)]
    };

    // 组合卡片内容
    let content = row![leading_icon].spacing(6).align_y(Alignment::Center).push(title);

    // 构建工具提示内容
    let tooltip_location_text = parsed_ref.location.map(format_reference_location);
    let tooltip_tip = container({
        let mut tip = iced::widget::column![
            text(format!("@{}", parsed_ref.path))
                .size(12)
                .style(|_theme: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) })
        ]
        .spacing(2);

        // 如果有位置信息，添加到提示中
        if let Some(location) = tooltip_location_text {
            tip = tip.push(text(location).size(11).style(|_theme: &Theme| {
                iced::widget::text::Style { color: Some(Color::WHITE.scale_alpha(0.78)) }
            }));
        }

        tip
    })
    .style(tooltip_dark_style)
    .padding([6, 8]);

    // 构建卡片容器并添加工具提示
    let card = container(content)
        .padding([5.0, 8.0])
        .width(Length::Shrink)
        .style(move |theme: &Theme| file_reference_style(theme, is_hovered));

    tooltip(card, tooltip_tip, Position::Top).into()
}

/// 从文本中提取所有文件引用
///
/// 扫描文本中所有 `@` 开头的文件引用，支持路径中包含：
/// - 字母数字
/// - 路径分隔符 (`/`, `\`)
/// - 文件扩展名点 (`.`)
/// - 下划线和连字符 (`_`, `-`)
/// - 冒号（用于行列位置）(`:`)
/// - 井号（用于锚点）(`#`)
///
/// # 参数
///
/// - `text` - 待扫描的文本内容
///
/// # 返回值
///
/// 返回提取到的文件引用列表（不含 `@` 符号）
///
/// # 示例
///
/// ```ignore
/// let text = "请查看 @src/main.rs:10 和 @lib/utils.rs:5-15";
/// let mentions = extract_file_mentions(text);
/// assert_eq!(mentions, vec!["src/main.rs:10", "lib/utils.rs:5-15"]);
/// ```
pub fn extract_file_mentions(text: &str) -> Vec<String> {
    let mut files = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0usize;

    // 逐字节扫描文本
    while i < bytes.len() {
        // 发现 @ 符号，开始提取引用
        if bytes[i] == b'@' {
            let start = i;
            i += 1;
            // 继续读取直到遇到无效字符
            while i < bytes.len() {
                let b = bytes[i];
                // 检查字符是否有效
                let is_valid = b.is_ascii_alphanumeric()
                    || matches!(b, b'/' | b'\\' | b'.' | b'_' | b'-' | b':' | b'#');
                if !is_valid {
                    break;
                }
                i += 1;
            }

            // 确保至少有一个字符（不含 @）
            if i > start + 1 {
                let mention = &text[start + 1..i];
                if !mention.is_empty() {
                    files.push(mention.to_string());
                }
            }
            continue;
        }
        i += 1;
    }

    files
}

/// 渲染文件引用卡片列表
///
/// 将文件引用列表渲染为一行可换行的卡片组件。
/// 每个卡片支持悬停状态显示和删除操作。
///
/// # 参数
///
/// - `file_mentions` - 文件引用字符串列表
/// - `hovered_index` - 当前悬停的卡片索引（None 表示无悬停）
///
/// # 返回值
///
/// 返回渲染好的 Element 组件，如果列表为空则返回空白组件
pub fn render_file_references<'a>(
    file_mentions: &[String],
    hovered_index: Option<usize>,
) -> Element<'a, Message> {
    // 空列表返回空白组件
    if file_mentions.is_empty() {
        return iced::widget::Space::new().into();
    }

    // 构建卡片行
    let mut refs_row = row![].spacing(6);
    for (idx, file_path) in file_mentions.iter().enumerate() {
        let hovered = hovered_index == Some(idx);
        let card = file_reference_card(file_path.clone(), hovered);
        // 包装鼠标区域以处理悬停事件
        let card = mouse_area(card)
            .on_enter(Message::Chat(message::ChatMessage::FileReferenceHoverChanged(Some(idx))))
            .on_exit(Message::Chat(message::ChatMessage::FileReferenceHoverChanged(None)));
        refs_row = refs_row.push(card);
    }
    // 使用 wrap 实现自动换行
    container(refs_row.wrap()).width(Length::Fill).padding([4, 0]).into()
}
