//! 将主题定义转换为 Iced `Theme::custom`。

use anyhow::Result;
use iced::Theme;
use iced::theme::Palette;

use super::model::{ThemeDefinition, parse_rrggbb};

/// 将已校验的主题定义转换为带名称的自定义 Iced 主题。
pub fn to_iced_theme(definition: &ThemeDefinition) -> Result<Theme> {
    definition.validate()?;
    let palette = Palette {
        background: parse_rrggbb(&definition.palette.background)?,
        text: parse_rrggbb(&definition.palette.text)?,
        primary: parse_rrggbb(&definition.palette.primary)?,
        success: parse_rrggbb(&definition.palette.success)?,
        warning: parse_rrggbb(&definition.palette.warning)?,
        danger: parse_rrggbb(&definition.palette.danger)?,
    };
    Ok(Theme::custom(definition.name.clone(), palette))
}
