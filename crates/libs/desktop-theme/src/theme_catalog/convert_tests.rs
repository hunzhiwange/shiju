//! 主题转换测试，验证目录主题的十六进制调色板准确映射到 Iced 主题。
//! 该边界保证自定义色值进入渲染层时不被内置主题默认色覆盖。

use iced::theme::Palette;
use iced::{Color, Theme};

use super::{ThemeAssets, ThemeChrome, ThemeDefinition, ThemePaletteHex, to_iced_theme};

#[test]
fn theme_definition_to_iced_theme_uses_custom_palette() {
    let definition = ThemeDefinition {
        id: "mint-night".to_string(),
        name: "Mint Night".to_string(),
        tagline: None,
        palette: ThemePaletteHex {
            background: "#0b1220".to_string(),
            text: "#e2e8f0".to_string(),
            primary: "#34d399".to_string(),
            success: "#22c55e".to_string(),
            warning: "#f59e0b".to_string(),
            danger: "#ef4444".to_string(),
        },
        assets: ThemeAssets::default(),
        chrome: ThemeChrome::default(),
    };

    let theme = to_iced_theme(&definition).expect("convert");
    assert!(matches!(theme, Theme::Custom(_)));
    assert_eq!(theme.to_string(), "Mint Night");

    let palette = theme.palette();
    assert_eq!(
        palette,
        Palette {
            background: Color::from_rgb8(0x0b, 0x12, 0x20),
            text: Color::from_rgb8(0xe2, 0xe8, 0xf0),
            primary: Color::from_rgb8(0x34, 0xd3, 0x99),
            success: Color::from_rgb8(0x22, 0xc5, 0x5e),
            warning: Color::from_rgb8(0xf5, 0x9e, 0x0b),
            danger: Color::from_rgb8(0xef, 0x44, 0x44),
        }
    );
}
