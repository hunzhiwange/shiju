//! 主题模型兼容门面与桌面颜色转换。

pub use theme_model::{ThemeAssets, ThemeChrome, ThemeDefinition, ThemePaletteHex};

use anyhow::Result;
use theme_model::validate_hex_color;

pub(super) fn parse_rrggbb(value: &str) -> Result<iced::Color> {
    validate_hex_color("color", value)?;
    let r = u8::from_str_radix(&value[1..3], 16)?;
    let g = u8::from_str_radix(&value[3..5], 16)?;
    let b = u8::from_str_radix(&value[5..7], 16)?;
    Ok(iced::Color::from_rgb8(r, g, b))
}
