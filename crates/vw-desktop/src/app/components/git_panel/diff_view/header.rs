//! Diff 视图头部组件模块
//!
//! 本模块提供 Git diff 视图的头部构建功能，包括：
//! - 构建包含标题和增删统计的头部内容
//! - 根据增删行数动态设置头部背景颜色
//! - 支持可选的关闭按钮
//!
//! 头部显示效果：
//! - 仅删除：淡红色背景
//! - 仅新增：淡绿色背景
//! - 新增+删除：淡黄色背景
//! - 无变化：默认背景

use iced::widget::{Space, container, row, text};
use iced::{Background, Border, Color, Element, Length};

use crate::app::Message;
use crate::app::assets::Icon;

use super::super::ui::{header_plain_glyph_button, square_icon_button_tiny};

pub(super) fn mix_color(a: Color, b: Color, ratio: f32) -> Color {
    let ratio = ratio.clamp(0.0, 1.0);
    Color {
        r: a.r + (b.r - a.r) * ratio,
        g: a.g + (b.g - a.g) * ratio,
        b: a.b + (b.b - a.b) * ratio,
        a: a.a + (b.a - a.a) * ratio,
    }
}

pub(super) fn clean_diff_header_title(
    title: &str,
    plus_token: &str,
    minus_token: &str,
    insertions: usize,
    deletions: usize,
) -> String {
    let mut title_display = title.to_string();
    let strip_token = |s: &mut String, token: &str| {
        *s = s.replace(&format!("({token})"), "");
        *s = s.replace(&format!("（{token}）"), "");
        *s = s.replace(&format!("{token} "), "");
        *s = s.replace(&format!(" {token}"), "");
        *s = s.replace(token, "");
    };

    if insertions > 0 {
        strip_token(&mut title_display, plus_token);
    }

    if deletions > 0 {
        strip_token(&mut title_display, minus_token);
    }

    title_display.split_whitespace().collect::<Vec<&str>>().join(" ")
}

pub(super) fn diff_stat_badge_text_style(
    theme: &iced::Theme,
    positive: bool,
) -> iced::widget::text::Style {
    let ext = theme.extended_palette();
    let pair = if positive { ext.success.base } else { ext.danger.base };
    iced::widget::text::Style { color: Some(pair.color) }
}

pub(super) fn diff_stat_badge_container_style(
    theme: &iced::Theme,
    positive: bool,
) -> iced::widget::container::Style {
    let ext = theme.extended_palette();
    let pair = if positive { ext.success.base } else { ext.danger.base };
    iced::widget::container::Style { text_color: Some(pair.color), ..Default::default() }
}

pub(super) fn diff_header_title_text_style(theme: &iced::Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(theme.palette().text.scale_alpha(0.92)) }
}

pub(super) fn is_dark_diff_header_theme(theme: &iced::Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}

pub(super) fn diff_header_accent(
    theme: &iced::Theme,
    insertions: usize,
    deletions: usize,
) -> Option<Color> {
    let ext = theme.extended_palette();
    if deletions > 0 && insertions == 0 {
        Some(ext.danger.base.color)
    } else if insertions > 0 && deletions == 0 {
        Some(ext.success.base.color)
    } else if insertions > 0 && deletions > 0 {
        Some(mix_color(ext.success.base.color, ext.danger.base.color, 0.5))
    } else {
        None
    }
}

pub(super) fn diff_header_container_style(
    theme: &iced::Theme,
    insertions: usize,
    deletions: usize,
) -> iced::widget::container::Style {
    let ext = theme.extended_palette();
    let is_dark = is_dark_diff_header_theme(theme);
    let neutral_bg = if is_dark {
        ext.background.weak.color.scale_alpha(0.24)
    } else {
        ext.background.base.color.scale_alpha(0.98)
    };
    let neutral_border = ext.background.strong.color.scale_alpha(if is_dark { 0.30 } else { 0.18 });
    let (background, border_color) = if let Some(accent) =
        diff_header_accent(theme, insertions, deletions)
    {
        (
            mix_color(neutral_bg, accent.scale_alpha(if is_dark { 0.12 } else { 0.08 }), 0.28),
            mix_color(neutral_border, accent.scale_alpha(if is_dark { 0.24 } else { 0.16 }), 0.22),
        )
    } else {
        (neutral_bg, neutral_border)
    };
    iced::widget::container::Style {
        background: Some(Background::Color(background)),
        border: Border { width: 1.0, color: border_color, radius: 10.0.into() },
        ..Default::default()
    }
}

fn diff_stat_badge<'a>(token: String, positive: bool) -> Element<'a, Message> {
    container(
        text(token)
            .size(11)
            .line_height(iced::widget::text::LineHeight::Relative(1.0))
            .style(move |theme: &iced::Theme| diff_stat_badge_text_style(theme, positive)),
    )
    .padding([0, 2])
    .style(move |theme: &iced::Theme| diff_stat_badge_container_style(theme, positive))
    .into()
}

/// 构建 diff 视图的头部组件
///
/// 该函数创建一个包含以下元素的头部：
/// - 文件标题（自动清理已存在的统计信息）
/// - 新增行数统计（绿色文本，如 "+5"）
/// - 删除行数统计（红色文本，如 "-3"）
/// - 可选的关闭按钮
///
/// # 参数
///
/// * `title` - 文件标题字符串，可能包含已有的统计信息（如 "(+5)"）
/// * `insertions` - 新增行数
/// * `deletions` - 删除行数
/// * `close_message` - 可选的关闭消息，存在时显示关闭按钮
///
/// # 返回值
///
/// 返回构建好的头部 `Element`，可嵌入到 iced 布局中
///
/// # 示例
///
/// ```ignore
/// let header = build_diff_header(
///     "src/main.rs (+5 -3)".to_string(),
///     5,
///     3,
///     Some(Message::CloseDiff),
/// );
/// ```
pub fn build_diff_header(
    title: String,
    insertions: usize,
    deletions: usize,
    close_message: Option<Message>,
    fullscreen_message: Option<Message>,
    fullscreen_tip: Option<String>,
    fullscreen_icon: Option<Icon>,
) -> Element<'static, Message> {
    // 格式化增删统计标记
    let plus_token = format!("+{}", insertions);
    let minus_token = format!("-{}", deletions);

    let title_display =
        clean_diff_header_title(&title, &plus_token, &minus_token, insertions, deletions);

    // 判断是否需要显示统计信息
    let show_plus = insertions > 0;
    let show_minus = deletions > 0;

    // 构建统计信息区域（新增/删除行数显示）
    let stats: Option<Element<'static, Message>> = if show_plus || show_minus {
        let mut r = row![].spacing(5).align_y(iced::Alignment::Center);

        // 添加新增行数统计（绿色）
        if show_plus {
            r = r.push(diff_stat_badge(plus_token.clone(), true));
        }

        // 添加删除行数统计（红色）
        if show_minus {
            r = r.push(diff_stat_badge(minus_token.clone(), false));
        }
        Some(r.into())
    } else {
        None
    };

    // 构建完整的头部内容
    let fullscreen_button: Option<Element<'static, Message>> = fullscreen_message
        .zip(fullscreen_tip)
        .zip(fullscreen_icon)
        .map(|((message, tip), icon)| square_icon_button_tiny(icon, tip, message));

    let header_content: Element<'static, Message> = if let Some(close_message) = close_message {
        // 有关闭按钮的情况：标题 + 统计 + 填充空间 + 关闭按钮
        let title = container(
            text(title_display)
                .size(12)
                .line_height(iced::widget::text::LineHeight::Relative(1.0))
                .wrapping(iced::widget::text::Wrapping::None)
                .style(diff_header_title_text_style),
        )
        .width(Length::Fill)
        .clip(true);

        let mut r = row![title].spacing(10).width(Length::Fill).align_y(iced::Alignment::Center);

        // 添加统计信息（如果存在）
        if let Some(stats) = stats {
            r = r.push(stats);
        }

        // 添加弹性空间、全屏按钮和关闭按钮
        r = r.push(Space::new().width(Length::Fill));
        if let Some(fullscreen_button) = fullscreen_button {
            r = r.push(fullscreen_button);
        }
        r = r.push(header_plain_glyph_button("✕", "关闭".to_string(), close_message));
        r.into()
    } else {
        // 无关闭按钮的情况：仅标题 + 统计
        let title = container(
            text(title_display)
                .size(12)
                .line_height(iced::widget::text::LineHeight::Relative(1.0))
                .wrapping(iced::widget::text::Wrapping::None)
                .style(diff_header_title_text_style),
        )
        .width(Length::Fill)
        .clip(true);

        let mut r = row![title].spacing(10).width(Length::Fill).align_y(iced::Alignment::Center);

        // 添加统计信息（如果存在）
        if let Some(stats) = stats {
            r = r.push(stats);
        }
        r = r.push(Space::new().width(Length::Fill));
        if let Some(fullscreen_button) = fullscreen_button {
            r = r.push(fullscreen_button);
        }
        r.into()
    };

    header_content
}

/// 为 diff 头部内容添加容器样式包装
///
/// 该函数根据增删行数的比例，动态设置头部容器的背景颜色和边框：
/// - 仅删除：淡红色背景和边框（表示危险/移除操作）
/// - 仅新增：淡绿色背景和边框（表示安全/添加操作）
/// - 新增+删除：淡黄色背景和边框（表示混合修改）
/// - 无变化：使用主题默认背景和边框
///
/// # 参数
///
/// * `header_content` - 由 `build_diff_header` 构建的头部内容元素
/// * `insertions` - 新增行数，用于决定背景颜色
/// * `deletions` - 删除行数，用于决定背景颜色
///
/// # 返回值
///
/// 返回包装后的 `Element`，带有动态背景和边框样式
///
/// # 示例
///
/// ```ignore
/// let header = build_diff_header(title, 5, 3, None);
/// let styled_header = wrap_diff_header(header, 5, 3);
/// ```
pub fn wrap_diff_header(
    header_content: Element<'static, Message>,
    insertions: usize,
    deletions: usize,
) -> Element<'static, Message> {
    container(header_content)
        .padding([8, 10])
        .width(Length::Fill)
        .style(move |theme: &iced::Theme| diff_header_container_style(theme, insertions, deletions))
        .into()
}
