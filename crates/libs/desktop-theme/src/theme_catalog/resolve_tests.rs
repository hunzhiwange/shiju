//! 主题解析测试，验证自定义主题标识从存储加载并在启动阶段生效。
//! 解析结果需保留目录调色板，确保配置值与最终 Iced 主题一致。

use iced::theme::Palette;
use iced::{Color, Theme};

use super::{
    ThemeAssets, ThemeChrome, ThemeDefinition, ThemePaletteHex, ThemeStore, resolve_app_theme_name,
    to_iced_theme,
};

fn sample_definition(id: &str, name: &str) -> ThemeDefinition {
    ThemeDefinition {
        id: id.to_string(),
        name: name.to_string(),
        tagline: Some("bootstrap recovery palette".to_string()),
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
    }
}

#[test]
fn resolve_custom_theme_id_loads_store_theme() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = ThemeStore::new(temp.path());
    let definition = sample_definition("mint-night", "Mint Night");
    store.save(&definition).expect("save");

    let theme = resolve_app_theme_name("custom:mint-night", Some(&store))
        .expect("custom theme should resolve");

    assert!(matches!(theme, Theme::Custom(_)));
    assert_eq!(theme.to_string(), "Mint Night");
    assert_eq!(
        theme.palette(),
        Palette {
            background: Color::from_rgb8(0x0b, 0x12, 0x20),
            text: Color::from_rgb8(0xe2, 0xe8, 0xf0),
            primary: Color::from_rgb8(0x34, 0xd3, 0x99),
            success: Color::from_rgb8(0x22, 0xc5, 0x5e),
            warning: Color::from_rgb8(0xf5, 0x9e, 0x0b),
            danger: Color::from_rgb8(0xef, 0x44, 0x44),
        }
    );

    assert!(
        resolve_app_theme_name("custom:missing-id", Some(&store)).is_none(),
        "unknown custom id must not resolve"
    );
    assert!(
        resolve_app_theme_name("Mint Night", Some(&store)).is_none(),
        "display name must not silently match a custom theme"
    );
    assert_eq!(resolve_app_theme_name("Dark", Some(&store)), Some(Theme::Dark));
}

#[test]
fn bootstrap_resolves_custom_app_theme() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = ThemeStore::new(temp.path());
    let definition = sample_definition("ocean-dusk", "Ocean Dusk");
    store.save(&definition).expect("save");

    let expected = to_iced_theme(&definition).expect("convert");
    let resolved = resolve_app_theme_name("custom:ocean-dusk", Some(&store))
        .expect("bootstrap should restore custom app theme");

    assert_eq!(resolved.to_string(), expected.to_string());
    assert_eq!(resolved.palette(), expected.palette());
    assert!(matches!(resolved, Theme::Custom(_)));
}
