use std::fs;
use std::path::{Path, PathBuf};

use super::{ActiveThemeSkin, resolve_active_theme_skin};
use crate::app::theme_catalog::{
    ThemeAssetSources, ThemeAssets, ThemeChrome, ThemeDefinition, ThemePaletteHex, ThemeStore,
};

fn sample_definition(id: &str) -> ThemeDefinition {
    ThemeDefinition {
        id: id.to_string(),
        name: "Wall Theme".to_string(),
        tagline: None,
        palette: ThemePaletteHex {
            background: "#0b1220".to_string(),
            text: "#e2e8f0".to_string(),
            primary: "#38bdf8".to_string(),
            success: "#22c55e".to_string(),
            warning: "#f59e0b".to_string(),
            danger: "#ef4444".to_string(),
        },
        assets: ThemeAssets::default(),
        chrome: ThemeChrome { input_bg: Some("#111827".to_string()), ..ThemeChrome::default() },
    }
}

fn minimal_png_bytes() -> &'static [u8] {
    &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x08, 0xd7, 0x63, 0xf8,
        0xff, 0xff, 0x3f, 0x00, 0x05, 0xfe, 0x02, 0xfe, 0xdc, 0xcc, 0x59, 0xe7, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ]
}

fn write_fixture(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, bytes).expect("write fixture");
    path
}

#[test]
fn resolve_active_theme_skin_sets_background_path_and_chrome() {
    let temp = tempfile::tempdir().expect("tempdir");
    let fixtures = temp.path().join("fixtures");
    fs::create_dir_all(&fixtures).expect("fixtures");
    let background_src = write_fixture(&fixtures, "wall.png", minimal_png_bytes());

    let store = ThemeStore::new(temp.path().join("themes"));
    let saved = store
        .save_with_sources(
            &sample_definition("wall-night"),
            &ThemeAssetSources {
                cover: None,
                background: Some(background_src),
                ..ThemeAssetSources::default()
            },
        )
        .expect("save");

    let skin = resolve_active_theme_skin(&store, "wall-night").expect("resolve");
    assert_eq!(skin.theme_id, "wall-night");
    assert_eq!(
        skin.background_path.as_ref().map(|p| p.file_name().and_then(|n| n.to_str())),
        Some(Some("background.png"))
    );
    assert!(
        skin.background_path.as_ref().is_some_and(|p| p.is_file()),
        "background file must exist"
    );
    assert_eq!(skin.chrome.input_bg.as_deref(), Some("#111827"));
    assert_eq!(saved.assets.background.as_deref(), Some("background.png"));
}

#[test]
fn resolve_active_theme_skin_missing_background_file_degrades() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = ThemeStore::new(temp.path().join("themes"));
    let mut definition = sample_definition("missing-wall");
    definition.assets.background = Some("background.png".to_string());
    store.save(&definition).expect("save json without image file");

    let skin = resolve_active_theme_skin(&store, "missing-wall").expect("resolve");
    assert!(skin.background_path.is_none());
    assert_eq!(skin.chrome.input_bg.as_deref(), Some("#111827"));
}

#[test]
fn active_theme_skin_from_definition_without_assets() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = ThemeStore::new(temp.path().join("themes"));
    let definition = sample_definition("plain");
    store.save(&definition).expect("save");
    let skin = ActiveThemeSkin::from_definition(&store, &definition);
    assert!(skin.background_path.is_none());
    assert!(skin.cover_path.is_none());
    assert!(skin.chat_header_path.is_none());
    assert!(skin.chat_body_path.is_none());
    assert!(skin.chat_input_path.is_none());
}

#[test]
fn active_theme_skin_resolves_chat_header_path() {
    let temp = tempfile::tempdir().expect("tempdir");
    let fixtures = temp.path().join("fixtures");
    fs::create_dir_all(&fixtures).expect("fixtures");
    let header_src = write_fixture(&fixtures, "header.png", minimal_png_bytes());

    let store = ThemeStore::new(temp.path().join("themes"));
    let saved = store
        .save_with_sources(
            &sample_definition("header-skin"),
            &ThemeAssetSources {
                cover: None,
                background: None,
                chat_header: Some(header_src),
                chat_body: None,
                chat_input: None,
            },
        )
        .expect("save");

    let skin = resolve_active_theme_skin(&store, "header-skin").expect("resolve");
    assert_eq!(
        skin.chat_header_path.as_ref().map(|p| p.file_name().and_then(|n| n.to_str())),
        Some(Some("chat_header.png"))
    );
    assert!(
        skin.chat_header_path.as_ref().is_some_and(|p| p.is_file()),
        "chat header file must exist"
    );
    assert!(skin.chat_body_path.is_none());
    assert!(skin.chat_input_path.is_none());
    assert_eq!(saved.assets.chat_header.as_deref(), Some("chat_header.png"));
}

#[test]
fn active_theme_skin_missing_chat_header_file_degrades() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = ThemeStore::new(temp.path().join("themes"));
    let mut definition = sample_definition("missing-header");
    definition.assets.chat_header = Some("chat_header.png".to_string());
    store.save(&definition).expect("save json without image file");

    let skin = resolve_active_theme_skin(&store, "missing-header").expect("resolve");
    assert!(skin.chat_header_path.is_none());
    assert!(skin.chat_body_path.is_none());
    assert!(skin.chat_input_path.is_none());
}
