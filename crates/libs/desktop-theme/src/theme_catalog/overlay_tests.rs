//! 内置主题覆盖层测试，验证补丁保存、解析应用和恢复默认流程。
//! 覆盖标识必须稳定有效，恢复操作只清除用户补丁而不改写内置主题。

use iced::Theme;

use super::overlay::{
    builtin_overlay_slug, delete_builtin_overlay, load_builtin_overlay, save_builtin_overlay,
};
use super::{
    ThemeAssets, ThemeChrome, ThemeDefinition, ThemePaletteHex, resolve_app_theme_name,
    set_test_themes_root,
};

fn overlay_definition(slug: &str, name: &str, primary: &str) -> ThemeDefinition {
    ThemeDefinition {
        id: slug.to_string(),
        name: name.to_string(),
        tagline: None,
        palette: ThemePaletteHex {
            background: "#111111".to_string(),
            text: "#eeeeee".to_string(),
            primary: primary.to_string(),
            success: "#22c55e".to_string(),
            warning: "#f59e0b".to_string(),
            danger: "#ef4444".to_string(),
        },
        assets: ThemeAssets::default(),
        chrome: ThemeChrome::default(),
    }
}

fn with_temp_themes_root<T>(f: impl FnOnce(std::path::PathBuf) -> T) -> T {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();
    set_test_themes_root(Some(root.clone()));
    let result = f(root);
    set_test_themes_root(None);
    result
}

#[test]
fn builtin_overlay_slug_is_stable_and_valid() {
    let slug = builtin_overlay_slug("Catppuccin Mocha");
    assert_eq!(slug, "catppuccin-mocha");
    assert_eq!(builtin_overlay_slug("Dark"), "dark");
}

#[test]
fn builtin_overlay_save_and_resolve_applies_patch() {
    with_temp_themes_root(|root| {
        let builtin_name = Theme::Dark.to_string();
        let slug = builtin_overlay_slug(&builtin_name);
        let patched = overlay_definition(&slug, &builtin_name, "#14b8a6");

        save_builtin_overlay(&root, &builtin_name, &patched).expect("save overlay");

        let overlay_path = root.join("overlays").join(&slug).join("theme.json");
        assert!(
            overlay_path.is_file(),
            "overlay theme.json should exist at {}",
            overlay_path.display()
        );

        let loaded =
            load_builtin_overlay(&root, &builtin_name).expect("load").expect("overlay present");
        assert_eq!(loaded.palette.primary, "#14b8a6");

        let resolved = resolve_app_theme_name(&builtin_name, None).expect("resolve builtin");
        assert_eq!(resolved.to_string(), builtin_name);
        assert_eq!(
            resolved.palette().primary,
            iced::Color::from_rgb8(0x14, 0xb8, 0xa6),
            "resolve must apply overlay primary color"
        );
        assert_ne!(
            resolved.palette(),
            Theme::Dark.palette(),
            "overlaid palette must differ from pure builtin"
        );
    });
}

#[test]
fn builtin_overlay_restore_clears_patch() {
    with_temp_themes_root(|root| {
        let builtin_name = Theme::Dark.to_string();
        let slug = builtin_overlay_slug(&builtin_name);
        let patched = overlay_definition(&slug, &builtin_name, "#14b8a6");
        save_builtin_overlay(&root, &builtin_name, &patched).expect("save overlay");

        delete_builtin_overlay(&root, &builtin_name).expect("restore default");

        let overlay_dir = root.join("overlays").join(&slug);
        assert!(!overlay_dir.exists(), "overlay directory must be removed after restore");
        assert!(
            load_builtin_overlay(&root, &builtin_name).expect("load").is_none(),
            "load must return None after restore"
        );

        let resolved = resolve_app_theme_name(&builtin_name, None).expect("resolve builtin");
        assert_eq!(resolved, Theme::Dark);
        assert_eq!(resolved.palette(), Theme::Dark.palette());
    });
}
