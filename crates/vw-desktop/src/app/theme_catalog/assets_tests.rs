use std::fs;
use std::path::{Path, PathBuf};

use super::assets::{
    ThemeAssetKind, copy_theme_asset, relative_asset_name, resolve_theme_asset_path,
    supported_image_extension,
};

/// 最小合法 1×1 PNG（含 IHDR/IDAT/IEND）。
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
fn supported_image_extension_accepts_png_jpg_jpeg_webp() {
    assert_eq!(supported_image_extension(Path::new("a.PNG")).as_deref(), Some("png"));
    assert_eq!(supported_image_extension(Path::new("a.jpg")).as_deref(), Some("jpg"));
    assert_eq!(supported_image_extension(Path::new("a.jpeg")).as_deref(), Some("jpeg"));
    assert_eq!(supported_image_extension(Path::new("a.webp")).as_deref(), Some("webp"));
    assert!(supported_image_extension(Path::new("a.gif")).is_none());
    assert!(supported_image_extension(Path::new("a.bmp")).is_none());
    assert!(supported_image_extension(Path::new("a")).is_none());
}

#[test]
fn relative_asset_name_uses_kind_and_source_extension() {
    assert_eq!(
        relative_asset_name(ThemeAssetKind::Cover, Path::new("/tmp/photo.JPEG")).unwrap(),
        "cover.jpeg"
    );
    assert_eq!(
        relative_asset_name(ThemeAssetKind::Background, Path::new("wall.webp")).unwrap(),
        "background.webp"
    );
    assert_eq!(
        relative_asset_name(ThemeAssetKind::ChatHeader, Path::new("h.PNG")).unwrap(),
        "chat_header.png"
    );
    assert_eq!(
        relative_asset_name(ThemeAssetKind::ChatBody, Path::new("b.jpg")).unwrap(),
        "chat_body.jpg"
    );
    assert_eq!(
        relative_asset_name(ThemeAssetKind::ChatInput, Path::new("i.webp")).unwrap(),
        "chat_input.webp"
    );
    let err = relative_asset_name(ThemeAssetKind::Cover, Path::new("x.gif")).unwrap_err();
    assert!(
        err.to_string().to_lowercase().contains("unsupported")
            || err.to_string().to_lowercase().contains("image"),
        "unexpected error: {err}"
    );
}

#[test]
fn copy_theme_asset_writes_bytes_under_theme_dir() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source = write_fixture(temp.path(), "src.png", minimal_png_bytes());
    let theme_dir = temp.path().join("ocean");
    fs::create_dir_all(&theme_dir).expect("theme dir");

    let relative = copy_theme_asset(ThemeAssetKind::Cover, &source, &theme_dir).expect("copy");
    assert_eq!(relative, "cover.png");
    let dest = theme_dir.join("cover.png");
    assert!(dest.is_file());
    assert_eq!(fs::read(&dest).expect("read dest"), minimal_png_bytes());
}

#[test]
fn resolve_theme_asset_path_joins_relative_name() {
    let root = PathBuf::from("/themes/ocean");
    let path = resolve_theme_asset_path(&root, "cover.png").expect("resolve");
    assert_eq!(path, PathBuf::from("/themes/ocean/cover.png"));

    assert!(resolve_theme_asset_path(&root, "../escape.png").is_err());
    assert!(resolve_theme_asset_path(&root, "/abs.png").is_err());
    assert!(resolve_theme_asset_path(&root, "sub/cover.png").is_err());
}
