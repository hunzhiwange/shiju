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
    let parts = title.split_whitespace();
    if insertions == 0 && deletions == 0 {
        return parts.collect::<Vec<_>>().join(" ");
    }

    let compact_stats = format!("{plus_token}{minus_token}");
    parts
        .filter(|part| {
            let token = part.trim_matches(['(', ')', '（', '）']);
            token != plus_token && token != minus_token && token != compact_stats
        })
        .collect::<Vec<_>>()
        .join(" ")
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

fn diff_stat_badge<'a, Message: 'a>(token: String, positive: bool) -> Element<'a, Message> {
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

/// 构建差异标题与增删统计，按钮由调用者提供以保留原有消息类型与样式。
pub fn build_diff_header<Message: 'static>(
    title: String,
    insertions: usize,
    deletions: usize,
    close_control: Option<Element<'static, Message>>,
    fullscreen_button: Option<Element<'static, Message>>,
) -> Element<'static, Message> {
    // 格式化增删统计标记
    let plus_token = format!("+{}", insertions);
    let minus_token = format!("-{}", deletions);

    let title_display =
        clean_diff_header_title(&title, &plus_token, &minus_token, insertions, deletions);

    let stats: Option<Element<'static, Message>> = (insertions > 0 || deletions > 0).then(|| {
        row![diff_stat_badge(plus_token, true), diff_stat_badge(minus_token, false)]
            .spacing(5)
            .align_y(iced::Alignment::Center)
            .into()
    });

    // 构建完整的头部内容
    let header_content: Element<'static, Message> = if let Some(close_control) = close_control {
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
        r = r.push(close_control);
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

/// 根据增删统计与明暗主题包装差异标题。
pub fn wrap_diff_header<Message: 'static>(
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
