//! 颜色选择器属性模块，负责颜色解析、格式转换和拾色控件渲染。

use iced::widget::text_input;
use iced::{Background, Theme};

mod conversion;
mod hsv;
mod images;
mod pickers;
mod render_full;
mod render_mini;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;

pub use conversion::{
    format_rgba_to_css, format_rgba_to_hex, hsla_to_rgba, parse_color, parse_css_color,
    rgba_to_hsla,
};
pub use hsv::Hsv;
pub use pickers::{AlphaPicker, HuePicker, SaturationValuePicker};
pub use render_full::render_color_picker;
pub use render_mini::render_mini_color_picker;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ColorFormat {
    Hex,
    Rgba,
    Hsl,
    Css,
}

impl std::fmt::Display for ColorFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorFormat::Hex => write!(f, "HEX"),
            ColorFormat::Rgba => write!(f, "RGBA"),
            ColorFormat::Hsl => write!(f, "HSL"),
            ColorFormat::Css => write!(f, "CSS"),
        }
    }
}

fn prop_text_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = theme.palette();
    let extended = theme.extended_palette();
    let focused = matches!(status, text_input::Status::Focused { .. });
    let border_color = if focused { palette.primary } else { extended.background.strong.color };
    let bg = if focused { extended.background.weak.color } else { extended.background.base.color };
    text_input::Style {
        background: Background::Color(bg),
        border: iced::Border { width: 1.0, color: border_color, radius: 8.0.into() },
        icon: palette.text.scale_alpha(0.5),
        placeholder: palette.text.scale_alpha(0.55),
        value: palette.text,
        selection: palette.primary.scale_alpha(0.30),
    }
}
