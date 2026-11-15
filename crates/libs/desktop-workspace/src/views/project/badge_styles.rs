use iced::{Background, Color, Theme};
use iced::border::{Border, Radius};
use iced::widget::container::Style as ContainerStyle;
fn blend_color(from: Color, to: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    Color {
        r: from.r + (to.r - from.r) * amount,
        g: from.g + (to.g - from.g) * amount,
        b: from.b + (to.b - from.b) * amount,
        a: from.a + (to.a - from.a) * amount,
    }
}
fn elevated_shadow(theme: &Theme, dark_alpha: f32, light_alpha: f32) -> iced::Shadow {
    iced::Shadow {
        color: Color::BLACK.scale_alpha(if is_dark_theme(theme) {
            dark_alpha
        } else {
            light_alpha
        }),
        offset: iced::Vector::new(0.0, 12.0),
        blur_radius: 28.0,
    }
}
pub fn is_dark_theme(theme: &Theme) -> bool {
    let palette = theme.palette();
    palette.background.r + palette.background.g + palette.background.b < 1.5
}
pub fn content_panel_style(theme: &Theme) -> ContainerStyle {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let background =
        desktop_theme::theme_catalog::panel_surface_color(theme, if is_dark { 0.98 } else { 0.985 });
    let border_color = if is_dark {
        palette.background.strong.color.scale_alpha(0.70)
    } else {
        palette.background.strong.color.scale_alpha(0.55)
    };

    ContainerStyle {
        background: Some(Background::Color(background)),
        border: Border { color: border_color, width: 1.0, radius: 18.0.into() },
        shadow: elevated_shadow(theme, 0.14, 0.04),
        ..Default::default()
    }
}
pub fn project_item_button_style(
    theme: &Theme,
    selected: bool,
    accent: Color,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let is_hovered = matches!(status, iced::widget::button::Status::Hovered);
    let is_pressed = matches!(status, iced::widget::button::Status::Pressed);
    let is_active = selected || is_hovered || is_pressed;

    let background = if is_pressed {
        blend_color(palette.background.base.color, accent, if is_dark { 0.26 } else { 0.18 })
    } else if is_hovered {
        blend_color(palette.background.base.color, accent, if is_dark { 0.16 } else { 0.11 })
    } else if selected {
        blend_color(palette.background.base.color, accent, if is_dark { 0.11 } else { 0.07 })
    } else {
        Color::TRANSPARENT
    };
    let border_color = if is_pressed {
        blend_color(accent, palette.background.base.color, 0.18)
    } else if is_hovered {
        blend_color(accent, palette.background.base.color, 0.28)
    } else if selected {
        blend_color(accent, palette.background.base.color, 0.36)
    } else {
        Color::TRANSPARENT
    };

    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color: theme.palette().text,
        border: iced::Border {
            radius: 18.0.into(),
            width: if is_active { 1.0 } else { 0.0 },
            color: border_color,
        },
        shadow: if is_active {
            iced::Shadow {
                color: accent.scale_alpha(if is_pressed {
                    if is_dark { 0.18 } else { 0.10 }
                } else if is_hovered {
                    if is_dark { 0.14 } else { 0.07 }
                } else if is_dark {
                    0.10
                } else {
                    0.05
                }),
                offset: iced::Vector::new(0.0, if is_pressed { 4.0 } else { 8.0 }),
                blur_radius: if is_pressed { 10.0 } else { 18.0 },
            }
        } else {
            iced::Shadow::default()
        },
        ..Default::default()
    }
}
pub fn panel_style_no_border(_theme: &Theme) -> ContainerStyle {
    ContainerStyle {
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: Radius::from(0.0) },
        ..Default::default()
    }
}
