//! 首页视图模块，负责项目入口、最近项目和常用工具入口的界面组合。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use iced::widget::image::Handle as ImageHandle;
use iced::widget::svg::Svg;
use iced::widget::{Image, button, container, text};
use iced::{Background, Border, Color, ContentFit, Element, Length, Theme, Vector};

use crate::app::Message;
use crate::app::assets::{self, Icon};
use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style,
};

/// 执行本模块的界面辅助逻辑。
///
/// # 参数
/// - `theme`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回判断结果，供调用方选择分支或样式。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub(super) fn is_dark_theme(theme: &Theme) -> bool {
    let palette = theme.palette();
    palette.background.r + palette.background.g + palette.background.b < 1.5
}

/// 执行本模块的界面辅助逻辑。
///
/// # 参数
/// - `icon`: 当前视图构建所需的状态、配置或消息。
/// - `size`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub(super) fn icon_svg(icon: Icon, size: f32) -> Svg<'static> {
    Svg::new(assets::get_icon(icon)).width(Length::Fixed(size)).height(Length::Fixed(size))
}

/// 执行本模块的界面辅助逻辑。
///
/// # 参数
/// - `label`: 当前视图构建所需的状态、配置或消息。
/// - `on`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub(super) fn primary_button<'a>(label: &'a str, on: Message) -> Element<'a, Message> {
    button(text(label).size(13).align_y(iced::alignment::Vertical::Center))
        .on_press(on)
        .height(Length::Fixed(30.0))
        .padding([0, 12])
        .style(|theme: &Theme, status| {
            let mut style = primary_action_btn_style(theme, status);
            style.border.radius = 12.0.into();
            style.shadow = iced::Shadow {
                color: theme.palette().primary.scale_alpha(0.18),
                offset: Vector::new(0.0, 10.0),
                blur_radius: 22.0,
            };
            style
        })
        .into()
}

/// 执行本模块的界面辅助逻辑。
///
/// # 参数
/// - `label`: 当前视图构建所需的状态、配置或消息。
/// - `on`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub(super) fn secondary_button<'a>(label: &'a str, on: Message) -> Element<'a, Message> {
    button(text(label).size(13).align_y(iced::alignment::Vertical::Center))
        .on_press(on)
        .height(Length::Fixed(30.0))
        .padding([0, 12])
        .style(|theme: &Theme, status| {
            let mut style = rounded_action_btn_style(theme, status);
            style.border.radius = 12.0.into();
            style
        })
        .into()
}

/// 解析输入值。
///
/// # 参数
/// - `input`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回匹配到的值；无法安全转换或当前状态不适用时返回 `None`。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub(in crate::app) fn parse_hex_color(input: &str) -> Option<Color> {
    let hex = input.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(hex.get(0..2)?, 16).ok()?;
    let g = u8::from_str_radix(hex.get(2..4)?, 16).ok()?;
    let b = u8::from_str_radix(hex.get(4..6)?, 16).ok()?;
    Some(Color::from_rgb8(r, g, b))
}

fn icon_image_handle(icon: &str) -> Option<ImageHandle> {
    let raw = icon.trim();
    if raw.is_empty() {
        return None;
    }

    let path_str =
        raw.strip_prefix("file:///").or_else(|| raw.strip_prefix("file://")).unwrap_or(raw);
    let path = std::path::Path::new(path_str);
    if path.exists() { Some(ImageHandle::from_path(path)) } else { None }
}

fn project_badge_label(title: &str) -> String {
    let mut first_non_ws = None;

    for ch in title.chars() {
        if ch.is_whitespace() {
            continue;
        }

        if first_non_ws.is_none() {
            first_non_ws = Some(ch);
        }

        if ch.is_alphanumeric() {
            return if ch.is_ascii_alphabetic() {
                ch.to_ascii_uppercase().to_string()
            } else {
                ch.to_string()
            };
        }
    }

    first_non_ws
        .map(|ch| {
            if ch.is_ascii_alphabetic() {
                ch.to_ascii_uppercase().to_string()
            } else {
                ch.to_string()
            }
        })
        .unwrap_or_else(|| "?".to_string())
}

fn stable_hash32(s: &str) -> u32 {
    let mut hash: u32 = 2166136261;
    for byte in s.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

/// 计算颜色表现。
///
/// # 参数
/// - `seed`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回根据输入和主题计算出的颜色。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub(super) fn project_accent_color(seed: &str) -> Color {
    let palette = [
        Color::from_rgb8(0x8A, 0x3F, 0xFF),
        Color::from_rgb8(0xFF, 0x4D, 0x7D),
        Color::from_rgb8(0x00, 0xA3, 0xFF),
        Color::from_rgb8(0xF2, 0xA9, 0x00),
        Color::from_rgb8(0x2E, 0xB8, 0x72),
        Color::from_rgb8(0xFF, 0x7A, 0x00),
        Color::from_rgb8(0x00, 0xC2, 0xB8),
        Color::from_rgb8(0xEF, 0x44, 0x44),
        Color::from_rgb8(0x3B, 0x82, 0xF6),
        Color::from_rgb8(0xA8, 0x55, 0xF7),
    ];

    let index = (stable_hash32(seed) as usize) % palette.len();
    palette[index]
}

/// 计算颜色表现。
///
/// # 参数
/// - `bg`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回根据输入和主题计算出的颜色。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub(super) fn contrast_text_color(bg: Color) -> Color {
    let lum = 0.2126 * bg.r + 0.7152 * bg.g + 0.0722 * bg.b;
    if lum > 0.62 { Color::from_rgb8(18, 18, 18) } else { Color::WHITE }
}

pub(super) fn home_emphasis_font() -> iced::Font {
    iced::Font {
        family: iced::font::Family::Name("JetBrains Mono"),
        weight: iced::font::Weight::Bold,
        ..Default::default()
    }
}

pub(super) fn home_cjk_emphasis_font() -> iced::Font {
    iced::Font {
        family: iced::font::Family::Name("Noto Sans CJK SC"),
        weight: iced::font::Weight::Bold,
        ..Default::default()
    }
}

fn lighten_color(color: Color) -> Color {
    Color {
        r: (color.r + 3.0) / 4.0,
        g: (color.g + 3.0) / 4.0,
        b: (color.b + 3.0) / 4.0,
        a: color.a,
    }
}

/// 执行本模块的界面辅助逻辑。
///
/// # 参数
/// - `title`: 当前视图构建所需的状态、配置或消息。
/// - `path`: 当前视图构建所需的状态、配置或消息。
/// - `custom_icon`: 当前视图构建所需的状态、配置或消息。
/// - `custom_color`: 当前视图构建所需的状态、配置或消息。
/// - `size`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub(in crate::app) fn project_avatar<'a>(
    title: &str,
    path: &str,
    custom_icon: Option<&str>,
    custom_color: Option<Color>,
    size: f32,
) -> Element<'a, Message> {
    let custom_icon_trimmed = custom_icon.map(str::trim).filter(|value| !value.is_empty());
    let image_handle = custom_icon_trimmed.and_then(icon_image_handle);
    let has_image_icon = image_handle.is_some();
    let text_icon = if has_image_icon {
        None
    } else {
        custom_icon_trimmed.and_then(|value| value.chars().next().map(|ch| ch.to_string()))
    };
    let label = text_icon.unwrap_or_else(|| project_badge_label(title));
    let accent = custom_color.unwrap_or_else(|| project_accent_color(path));
    let light_bg = lighten_color(accent);
    let use_custom_bg = custom_color.is_some() && custom_icon_trimmed.is_some() && !has_image_icon;
    let badge_bg = if use_custom_bg { accent } else { light_bg };
    let badge_text = if use_custom_bg { contrast_text_color(accent) } else { accent };
    let badge_font = if label.is_ascii() { home_emphasis_font() } else { home_cjk_emphasis_font() };

    let badge_content: Element<'a, Message> =
        match image_handle {
            Some(handle) => Image::new(handle)
                .content_fit(ContentFit::Fill)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            None => container(text(label).size((size * 0.36).round()).font(badge_font).style(
                move |_theme: &Theme| iced::widget::text::Style { color: Some(badge_text) },
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
        };

    container(badge_content)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .clip(has_image_icon)
        .style(move |theme: &Theme| {
            let border_color = if has_image_icon {
                accent.scale_alpha(if is_dark_theme(theme) { 0.38 } else { 0.24 })
            } else {
                lighten_color(accent)
            };

            iced::widget::container::Style {
                background: (!has_image_icon).then_some(Background::Color(badge_bg)),
                border: Border { width: 1.5, color: border_color, radius: (size * 0.34).into() },
                shadow: iced::Shadow {
                    color: Color::BLACK.scale_alpha(if has_image_icon { 0.12 } else { 0.08 }),
                    offset: Vector::new(0.0, 10.0),
                    blur_radius: 22.0,
                },
                text_color: Some(badge_text),
                ..Default::default()
            }
        })
        .into()
}
#[cfg(test)]
#[path = "common_tests.rs"]
mod common_tests;
