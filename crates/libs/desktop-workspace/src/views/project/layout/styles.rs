use iced::border::{Border, Radius};
use iced::widget::container::Style as ContainerStyle;
use iced::{Background, Color, Theme};
use super::super::badge_styles::is_dark_theme;
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
pub fn work_surface_style(theme: &Theme) -> ContainerStyle {
    let palette = theme.extended_palette();
    let background = if palette.is_dark {
        palette.background.weak.color
    } else {
        desktop_theme::theme_catalog::panel_surface_color(theme, 1.0)
    };
    ContainerStyle { background: Some(Background::Color(background)), ..Default::default() }
}
pub fn left_rail_style(_theme: &Theme) -> ContainerStyle {
    ContainerStyle {
        background: None,
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: Radius::from(0.0) },
        shadow: iced::Shadow::default(),
        ..Default::default()
    }
}
pub fn session_panel_style(theme: &Theme, corner_radius: f32) -> ContainerStyle {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let background = if is_dark {
        blend_color(palette.background.base.color, palette.background.strong.color, 0.28)
            .scale_alpha(0.99)
    } else {
        desktop_theme::theme_catalog::panel_surface_color(theme, 0.99)
    };
    let border_color = if is_dark {
        palette.background.strong.color.scale_alpha(0.78)
    } else {
        palette.background.strong.color.scale_alpha(0.55)
    };
    let radius = corner_radius + 4.0;

    ContainerStyle {
        background: Some(Background::Color(background)),
        border: Border { color: border_color, width: 1.0, radius: radius.into() },
        shadow: elevated_shadow(theme, 0.18, 0.05),
        ..Default::default()
    }
}
pub fn right_column_inner_style(theme: &Theme, corner_radius: f32) -> ContainerStyle {
    let palette = theme.extended_palette();
    let background = if is_dark_theme(theme) {
        blend_color(palette.background.base.color, palette.background.strong.color, 0.22)
            .scale_alpha(0.99)
    } else {
        desktop_theme::theme_catalog::panel_surface_color(theme, 0.99)
    };

    ContainerStyle {
        background: Some(Background::Color(background)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: (corner_radius + 4.0).into(),
        },
        ..Default::default()
    }
}

pub fn workspace_background_style(theme: &Theme) -> ContainerStyle {
    let palette = theme.extended_palette();
    let background = if is_dark_theme(theme) {
        blend_color(palette.background.base.color, Color::BLACK, 0.24)
    } else {
        blend_color(palette.background.base.color, Color::from_rgb8(244, 246, 250), 0.72)
    };

    ContainerStyle { background: Some(Background::Color(background)), ..Default::default() }
}

#[cfg(test)]
#[path="styles_tests.rs"] mod styles_tests;
