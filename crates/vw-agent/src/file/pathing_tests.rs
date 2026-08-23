// 覆盖路径包含关系与图像、二进制扩展名识别。

use super::pathing::{
    contains_path, image_mime_type, is_binary_by_extension, is_image_by_extension,
};
use std::path::Path;

// 临时目录夹具同时覆盖正常路径和逃逸路径，确保安全判断不依赖当前工作区。
#[test]
fn extension_helpers_are_case_insensitive() {
    assert!(is_image_by_extension("cover.PNG"));
    assert_eq!(image_mime_type("cover.jpeg"), "image/jpeg");
    assert!(is_binary_by_extension("archive.ZIP"));
    assert!(!is_binary_by_extension("src/lib.rs"));
}

// 缺失资源与越界资源的语义不同：前者可安全回退时仍不得放宽后者的拒绝策略。
#[test]
fn contains_path_rejects_parent_escape() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("root");
    std::fs::create_dir(&root).expect("create root");

    assert!(contains_path(&root, &root.join("child.txt")));
    assert!(!contains_path(&root, &temp.path().join("outside.txt")));
}

#[test]
fn contains_path_allows_missing_children_under_existing_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("root");
    let nested = root.join("nested");
    std::fs::create_dir_all(&nested).expect("create nested");

    assert!(contains_path(&root, &nested.join("new-file.txt")));
    assert!(!contains_path(&root, &temp.path().join("missing-parent/new-file.txt")));
}

#[test]
fn contains_path_falls_back_when_root_and_parent_do_not_exist() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("missing-root");

    assert!(contains_path(&root, &root.join("new-file.txt")));
    assert!(!contains_path(&root, Path::new("relative.txt")));
}

#[test]
fn contains_path_treats_empty_candidate_as_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("root");
    std::fs::create_dir(&root).expect("create root");

    assert!(contains_path(&root, Path::new("")));
}

#[test]
fn image_extension_helper_covers_known_formats() {
    let image_extensions = [
        "png", "jpg", "jpeg", "gif", "bmp", "webp", "ico", "tif", "tiff", "svg", "svgz", "avif",
        "apng", "jxl", "heic", "heif", "raw", "cr2", "nef", "arw", "dng", "orf", "raf", "pef",
        "x3f",
    ];

    for ext in image_extensions {
        assert!(is_image_by_extension(&format!("photo.{ext}")), "{ext} should be an image");
    }
    assert!(is_image_by_extension("PHOTO.PNG"));
    assert!(!is_image_by_extension("README"));
    assert!(!is_image_by_extension(".png"));
    assert!(!is_image_by_extension("archive.zip"));
}

#[test]
fn image_mime_type_maps_known_formats_and_falls_back_to_extension() {
    let cases = [
        ("icon.PNG", "image/png"),
        ("photo.jpg", "image/jpeg"),
        ("photo.jpeg", "image/jpeg"),
        ("anim.gif", "image/gif"),
        ("bitmap.bmp", "image/bmp"),
        ("photo.webp", "image/webp"),
        ("favicon.ico", "image/x-icon"),
        ("scan.tif", "image/tiff"),
        ("scan.tiff", "image/tiff"),
        ("vector.svg", "image/svg+xml"),
        ("vector.svgz", "image/svg+xml"),
        ("photo.avif", "image/avif"),
        ("anim.apng", "image/apng"),
        ("photo.jxl", "image/jxl"),
        ("photo.heic", "image/heic"),
        ("photo.heif", "image/heif"),
        ("photo.raw", "image/raw"),
        ("README", "image/"),
    ];

    for (path, expected) in cases {
        assert_eq!(image_mime_type(path), expected, "{path}");
    }
}

#[test]
fn binary_extension_helper_covers_known_formats() {
    let binary_extensions = [
        "exe",
        "dll",
        "pdb",
        "bin",
        "so",
        "dylib",
        "o",
        "a",
        "lib",
        "wav",
        "mp3",
        "ogg",
        "oga",
        "ogv",
        "ogx",
        "flac",
        "aac",
        "wma",
        "m4a",
        "weba",
        "mp4",
        "avi",
        "mov",
        "wmv",
        "flv",
        "webm",
        "mkv",
        "zip",
        "tar",
        "gz",
        "gzip",
        "bz",
        "bz2",
        "bzip",
        "bzip2",
        "7z",
        "rar",
        "xz",
        "lz",
        "z",
        "pdf",
        "doc",
        "docx",
        "ppt",
        "pptx",
        "xls",
        "xlsx",
        "dmg",
        "iso",
        "img",
        "vmdk",
        "ttf",
        "otf",
        "woff",
        "woff2",
        "eot",
        "sqlite",
        "db",
        "mdb",
        "apk",
        "ipa",
        "aab",
        "xapk",
        "app",
        "pkg",
        "deb",
        "rpm",
        "snap",
        "flatpak",
        "appimage",
        "msi",
        "msp",
        "jar",
        "war",
        "ear",
        "class",
        "kotlin_module",
        "dex",
        "vdex",
        "odex",
        "oat",
        "art",
        "wasm",
        "wat",
        "bc",
        "ll",
        "s",
        "ko",
        "sys",
        "drv",
        "efi",
        "rom",
        "com",
        "bat",
        "cmd",
        "ps1",
        "sh",
        "bash",
        "zsh",
        "fish",
    ];

    for ext in binary_extensions {
        assert!(is_binary_by_extension(&format!("asset.{ext}")), "{ext} should be binary");
    }
    assert!(is_binary_by_extension("ARCHIVE.ZIP"));
    assert!(!is_binary_by_extension("src/lib.rs"));
    assert!(!is_binary_by_extension("README"));
}
