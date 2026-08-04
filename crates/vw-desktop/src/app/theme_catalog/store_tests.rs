use std::fs;
use std::path::{Path, PathBuf};

use super::assets::ThemeAssetSources;
use super::{ThemeAssets, ThemeChrome, ThemeDefinition, ThemePaletteHex, ThemeStore};

fn sample_definition(id: &str, name: &str) -> ThemeDefinition {
    ThemeDefinition {
        id: id.to_string(),
        name: name.to_string(),
        tagline: Some("a calm local palette".to_string()),
        palette: ThemePaletteHex {
            background: "#1e1e2e".to_string(),
            text: "#cdd6f4".to_string(),
            primary: "#89b4fa".to_string(),
            success: "#a6e3a1".to_string(),
            warning: "#f9e2af".to_string(),
            danger: "#f38ba8".to_string(),
        },
        assets: ThemeAssets::default(),
        chrome: ThemeChrome::default(),
    }
}

/// 最小合法 1×1 PNG。
fn minimal_png_bytes() -> &'static [u8] {
    &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x08, 0xd7, 0x63, 0xf8,
        0xff, 0xff, 0x3f, 0x00, 0x05, 0xfe, 0x02, 0xfe, 0xdc, 0xcc, 0x59, 0xe7, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ]
}

/// 最小合法 JPEG。
fn minimal_jpeg_bytes() -> &'static [u8] {
    &[
        0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00,
        0x01, 0x00, 0x01, 0x00, 0x00, 0xff, 0xd9,
    ]
}

fn write_fixture(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, bytes).expect("write fixture");
    path
}

#[test]
fn theme_catalog_save_list_load_delete_roundtrip() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = ThemeStore::new(temp.path());

    let first = sample_definition("ocean-dusk", "Ocean Dusk");
    let second = sample_definition("forest", "Forest");

    store.save(&first).expect("save first");
    store.save(&second).expect("save second");

    let mut listed = store.list().expect("list");
    listed.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].id, "forest");
    assert_eq!(listed[1].id, "ocean-dusk");
    assert_eq!(listed[1].name, "Ocean Dusk");
    assert_eq!(listed[1].tagline.as_deref(), Some("a calm local palette"));
    assert_eq!(listed[1].palette.primary, "#89b4fa");

    let loaded = store.load("ocean-dusk").expect("load");
    assert_eq!(loaded, first);

    store.delete("ocean-dusk").expect("delete");
    let remaining = store.list().expect("list after delete");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, "forest");
    assert!(store.load("ocean-dusk").is_err());
}

#[test]
fn theme_catalog_rejects_invalid_hex_color() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = ThemeStore::new(temp.path());

    let mut invalid = sample_definition("bad-color", "Bad Color");
    invalid.palette.primary = "89b4fa".to_string();

    let err = store.save(&invalid).expect_err("hashless hex must be rejected");
    assert!(
        err.to_string().to_lowercase().contains("color")
            || err.to_string().to_lowercase().contains("hex")
            || err.to_string().contains("#rrggbb"),
        "unexpected error: {err}"
    );
    assert!(store.list().expect("list").is_empty());
}

#[test]
fn theme_store_save_copies_cover_and_background() {
    let temp = tempfile::tempdir().expect("tempdir");
    let fixtures = temp.path().join("fixtures");
    fs::create_dir_all(&fixtures).expect("fixtures dir");
    let cover_src = write_fixture(&fixtures, "photo.png", minimal_png_bytes());
    let background_src = write_fixture(&fixtures, "wall.jpg", minimal_jpeg_bytes());

    let store = ThemeStore::new(temp.path().join("themes"));
    let definition = sample_definition("ocean-dusk", "Ocean Dusk");
    let sources = ThemeAssetSources {
        cover: Some(cover_src),
        background: Some(background_src),
        ..ThemeAssetSources::default()
    };

    let saved = store.save_with_sources(&definition, &sources).expect("save with image sources");

    assert_eq!(saved.assets.cover.as_deref(), Some("cover.png"));
    assert_eq!(saved.assets.background.as_deref(), Some("background.jpg"));

    let theme_dir = temp.path().join("themes").join("ocean-dusk");
    assert!(theme_dir.join("theme.json").is_file());
    assert!(theme_dir.join("cover.png").is_file());
    assert!(theme_dir.join("background.jpg").is_file());
    assert_eq!(fs::read(theme_dir.join("cover.png")).expect("read cover"), minimal_png_bytes());
    assert_eq!(
        fs::read(theme_dir.join("background.jpg")).expect("read background"),
        minimal_jpeg_bytes()
    );

    let loaded = store.load("ocean-dusk").expect("load");
    assert_eq!(loaded.assets, saved.assets);

    store.delete("ocean-dusk").expect("delete");
    assert!(!theme_dir.exists(), "delete must remove theme directory and assets");
}

#[test]
fn theme_store_save_copies_chat_panel_assets() {
    let temp = tempfile::tempdir().expect("tempdir");
    let fixtures = temp.path().join("fixtures");
    fs::create_dir_all(&fixtures).expect("fixtures dir");
    let header_src = write_fixture(&fixtures, "header.png", minimal_png_bytes());
    let body_src = write_fixture(&fixtures, "body.jpg", minimal_jpeg_bytes());
    let input_src = write_fixture(&fixtures, "input.webp", minimal_png_bytes());

    let store = ThemeStore::new(temp.path().join("themes"));
    let definition = sample_definition("panel-skin", "Panel Skin");
    let sources = ThemeAssetSources {
        cover: None,
        background: None,
        chat_header: Some(header_src),
        chat_body: Some(body_src),
        chat_input: Some(input_src),
    };

    let saved =
        store.save_with_sources(&definition, &sources).expect("save with chat panel image sources");

    assert_eq!(saved.assets.chat_header.as_deref(), Some("chat_header.png"));
    assert_eq!(saved.assets.chat_body.as_deref(), Some("chat_body.jpg"));
    assert_eq!(saved.assets.chat_input.as_deref(), Some("chat_input.webp"));

    let theme_dir = temp.path().join("themes").join("panel-skin");
    assert!(theme_dir.join("chat_header.png").is_file());
    assert!(theme_dir.join("chat_body.jpg").is_file());
    assert!(theme_dir.join("chat_input.webp").is_file());

    let loaded = store.load("panel-skin").expect("load");
    assert_eq!(loaded.assets.chat_header, saved.assets.chat_header);
    assert_eq!(loaded.assets.chat_body, saved.assets.chat_body);
    assert_eq!(loaded.assets.chat_input, saved.assets.chat_input);
}

#[test]
fn theme_store_save_rejects_unsupported_image() {
    let temp = tempfile::tempdir().expect("tempdir");
    let fixtures = temp.path().join("fixtures");
    fs::create_dir_all(&fixtures).expect("fixtures dir");
    let bad_src = write_fixture(&fixtures, "cover.gif", b"GIF89a");

    let themes_root = temp.path().join("themes");
    let store = ThemeStore::new(&themes_root);
    let definition = sample_definition("bad-image", "Bad Image");
    let sources = ThemeAssetSources {
        cover: Some(bad_src),
        background: None,
        ..ThemeAssetSources::default()
    };

    let err = store.save_with_sources(&definition, &sources).expect_err("gif must be rejected");
    assert!(
        err.to_string().to_lowercase().contains("unsupported")
            || err.to_string().to_lowercase().contains("image")
            || err.to_string().to_lowercase().contains("gif"),
        "unexpected error: {err}"
    );

    assert!(
        !themes_root.join("bad-image").exists(),
        "failed save must not leave a partial theme directory"
    );
    assert!(store.list().expect("list").is_empty());
}
