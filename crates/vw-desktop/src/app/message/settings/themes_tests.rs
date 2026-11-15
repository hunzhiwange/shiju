use super::{update, ThemesMessage};
use crate::app::App;
use crate::app::message::settings::messages::SettingsMessage;
use crate::app::theme_catalog::{ThemeAssets, ThemeChrome, ThemeDefinition, ThemePaletteHex, ThemeStore, set_test_themes_root, to_iced_theme};
use iced::Theme;

fn test_app() -> App {
    App::new().0
}

fn sample_custom_theme() -> ThemeDefinition {
    ThemeDefinition {
        id: "mint-night".to_string(),
        name: "Mint Night".to_string(),
        tagline: Some("Cool mint accents".to_string()),
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

fn with_temp_themes_root<T>(f: impl FnOnce(&ThemeStore) -> T) -> T {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = ThemeStore::new(temp.path());
    set_test_themes_root(Some(temp.path().to_path_buf()));
    let result = f(&store);
    set_test_themes_root(None);
    result
}

#[test]
fn themes_apply_custom_updates_app_theme() {
    with_temp_themes_root(|store| {
        store.save(&sample_custom_theme()).expect("save");
        let expected = to_iced_theme(&sample_custom_theme()).expect("convert");

        let mut app = test_app();
        app.app_theme = Theme::Light;
        app.active_custom_theme_id = None;

        let _ = update(&mut app, SettingsMessage::Themes(ThemesMessage::Refresh));
        assert!(
            app.themes_settings.entries.iter().any(|entry| entry.id == "mint-night"),
            "refresh should load custom theme into gallery"
        );

        let _ = update(
            &mut app,
            SettingsMessage::Themes(ThemesMessage::ApplyCustom("mint-night".to_string())),
        );

        assert!(matches!(app.app_theme, Theme::Custom(_)));
        assert_eq!(app.app_theme.to_string(), "Mint Night");
        assert_eq!(app.app_theme.palette(), expected.palette());
        assert_eq!(app.active_custom_theme_id.as_deref(), Some("mint-night"));
    });
}
