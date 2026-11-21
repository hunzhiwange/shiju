//! 提供识矩应用页面在明暗主题下的组件样式。

use desktop_ui_controls::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, theme::is_dark_theme,
};
use iced::widget::{button, container, rule, svg, text};
use iced::{Background, Border, Color, Theme};

/// 识矩应用页面统一使用的滚动条宽度。
pub const SCROLLBAR_WIDTH: u32 = 4;
/// 滚动内容两侧预留空间。
pub const SCROLLABLE_SIDE_PADDING: u16 = 7;

const INK: Color = iced::color!(0x172948);
const MUTED: Color = iced::color!(0x5e6d84);
const PAGE: Color = iced::color!(0xf8f7f2);
const LINE: Color = iced::color!(0xe6e1d8);
const ACCENT: Color = iced::color!(0xd6633c);
const CARD: Color = iced::color!(0xfdfdfd);

fn color(theme: &Theme, light: Color, dark: Color) -> Color {
    if is_dark_theme(theme) { dark } else { light }
}

fn surface(theme: &Theme, light: Color, radius: f32, bordered: bool) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(color(theme, light, palette.background.weak.color).into()),
        text_color: Some(color(theme, INK, theme.palette().text)),
        border: Border {
            radius: radius.into(),
            width: if bordered { 1.0 } else { 0.0 },
            color: color(theme, LINE, palette.background.strong.color),
        },
        ..Default::default()
    }
}

pub fn page_style(theme: &Theme) -> container::Style {
    let mut style = surface(theme, PAGE, 0.0, false);
    if is_dark_theme(theme) {
        style.background = Some(theme.palette().background.into());
    }
    style
}

pub fn pill_track(theme: &Theme) -> container::Style {
    surface(theme, iced::color!(0xefede9), 12.0, false)
}

pub fn tab_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let mut style = rounded_action_btn_style(theme, status);
    let highlighted = selected || status == button::Status::Hovered;
    style.background = highlighted
        .then(|| color(theme, CARD, theme.extended_palette().background.strong.color).into());
    style.text_color = color(theme, if selected { INK } else { MUTED }, theme.palette().text);
    style.border = Border { radius: 9.0.into(), ..Default::default() };
    style
}

pub fn quiet_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = rounded_action_btn_style(theme, status);
    style.border.radius = 8.0.into();
    if !is_dark_theme(theme) {
        let (background, foreground) = match status {
            button::Status::Hovered | button::Status::Pressed => (iced::color!(0xf5e9df), INK),
            button::Status::Disabled => (CARD, iced::color!(0x8b96a6)),
            _ => (CARD, INK),
        };
        style.background = Some(background.into());
        style.text_color = foreground;
        style.border.color = LINE;
        style.border.width = 1.0;
    }
    style
}

pub fn primary_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = primary_action_btn_style(theme, status);
    style.border.radius = 8.0.into();
    if !is_dark_theme(theme) && status != button::Status::Disabled {
        style.background = Some(
            if status == button::Status::Active { ACCENT } else { iced::color!(0xb94c2a) }.into(),
        );
        style.text_color = Color::WHITE;
    }
    style
}

pub fn card_style(theme: &Theme) -> container::Style {
    surface(theme, CARD, 12.0, true)
}

pub fn panel_style(theme: &Theme) -> container::Style {
    surface(theme, CARD, 8.0, true)
}

pub fn table_header(theme: &Theme) -> container::Style {
    surface(theme, iced::color!(0xf3f0e9), 0.0, false)
}

pub fn app_icon_style(theme: &Theme) -> container::Style {
    surface(theme, iced::color!(0xf3f3f5), 9.0, false)
}

pub fn cover_ground(theme: &Theme) -> container::Style {
    surface(theme, iced::color!(0xdae2eb), 0.0, false)
}

pub fn cover_placeholder(theme: &Theme) -> container::Style {
    surface(theme, iced::color!(0xfeece6), 8.0, true)
}

pub fn divider_style(theme: &Theme) -> rule::Style {
    rule::Style {
        color: color(theme, LINE, theme.extended_palette().background.strong.color),
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

pub fn accent_icon(theme: &Theme, _: svg::Status) -> svg::Style {
    svg::Style { color: Some(color(theme, ACCENT, iced::color!(0xf08c68))) }
}

pub fn muted_icon(theme: &Theme, _: svg::Status) -> svg::Style {
    svg::Style { color: muted_text_style(theme).color }
}

/// 用成功色或弱化色区分应用是否运行就绪。
pub fn status_style(theme: &Theme, ready: bool) -> container::Style {
    let palette = theme.extended_palette();
    let pair = if ready { palette.success.weak } else { palette.danger.weak };
    container::Style {
        background: Some(Background::Color(pair.color)),
        text_color: Some(pair.text),
        border: Border { radius: 999.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub fn muted_text_style(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(color(theme, MUTED, theme.extended_palette().background.strong.text)),
    }
}

pub fn binding_field_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.base.color)),
        text_color: Some(theme.palette().text),
        border: Border { color: palette.background.strong.color, width: 1.0, radius: 9.0.into() },
        ..Default::default()
    }
}

/// 根据密钥绑定状态是否出错选择反馈容器颜色。
pub fn binding_state_style(theme: &Theme, error: bool) -> container::Style {
    let palette = theme.extended_palette();
    let pair = if error { palette.danger.weak } else { palette.primary.weak };
    container::Style {
        background: Some(Background::Color(pair.color)),
        text_color: Some(pair.text),
        border: Border { radius: 999.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub fn text_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::text(theme, status);
    style.text_color = color(
        theme,
        if status == button::Status::Hovered { ACCENT } else { INK },
        theme.palette().text,
    );
    style
}

pub fn lifecycle_style(theme: &Theme, status: &str) -> container::Style {
    if status == "已发布" {
        let mut style = surface(theme, iced::color!(0xebf2e8), 32.0, true);
        let success = theme.extended_palette().success.weak;
        style.text_color = Some(color(theme, iced::color!(0x466a37), success.text));
        style.border.color = color(theme, iced::color!(0xcbd9c1), success.color);
        style
    } else if status == "审核中" {
        surface(theme, iced::color!(0xfff2de), 32.0, true)
    } else {
        surface(theme, iced::color!(0xf4f2ec), 32.0, true)
    }
}

pub fn badge_style(theme: &Theme) -> container::Style {
    surface(theme, iced::color!(0xefede9), 24.0, false)
}
