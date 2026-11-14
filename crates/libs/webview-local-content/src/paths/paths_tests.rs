use super::*;
use crate::test_support::TestDir;
use std::fs;

#[test]
fn normalize_url_trims_nested_quotes() {
    assert_eq!(normalize_url("  `\"'https://example.com'\"`  "), "https://example.com");
    assert_eq!(normalize_url("  https://example.com  "), "https://example.com");
}

#[test]
fn normalize_url_keeps_unbalanced_quotes() {
    assert_eq!(normalize_url("'https://example.com"), "'https://example.com");
}

#[test]
fn percent_encode_url_path_keeps_unreserved_and_slash() {
    assert_eq!(percent_encode_url_path("/a b/中文/file.js"), "/a%20b/%E4%B8%AD%E6%96%87/file.js");
}

#[test]
fn percent_decode_decodes_hex_and_preserves_invalid_sequences() {
    assert_eq!(percent_decode("a%20b/%E4%B8%AD%E6%96%87"), "a b/中文");
    assert_eq!(percent_decode("%af"), "\u{FFFD}");
    assert_eq!(percent_decode("bad%2Gtail%"), "bad%2Gtail%");
}

#[test]
fn percent_decode_replaces_invalid_utf8() {
    assert_eq!(percent_decode("%FF"), "\u{FFFD}");
}

#[test]
fn file_url_to_path_decodes_localhost_url() {
    let path = file_url_to_path("file://localhost/tmp/a%20b/index.html").unwrap();

    #[cfg(not(target_os = "windows"))]
    assert_eq!(path, PathBuf::from("/tmp/a b/index.html"));
    #[cfg(target_os = "windows")]
    assert_eq!(path, PathBuf::from("tmp\\a b\\index.html"));
}

#[test]
fn resolve_target_keeps_http_urls() {
    assert_eq!(
        resolve_target("HTTPS://example.com"),
        StartTarget::Url("HTTPS://example.com".into())
    );
}

#[test]
fn resolve_target_uses_directory_index() {
    let dir = TestDir::new("resolve-dir");
    fs::write(dir.path().join("index.html"), "home").unwrap();

    let target = resolve_target(dir.path().to_str().unwrap());

    match target {
        StartTarget::LocalDir { root, entry } => {
            assert_eq!(root, fs::canonicalize(dir.path()).unwrap());
            assert_eq!(entry, "/index.html");
        }
        StartTarget::Url(_) => panic!("expected local directory"),
    }
}

#[test]
fn resolve_target_uses_file_parent_as_root() {
    let dir = TestDir::new("resolve-file");
    let file = dir.path().join("app.html");
    fs::write(&file, "app").unwrap();

    let target = resolve_target(file.to_str().unwrap());

    match target {
        StartTarget::LocalDir { root, entry } => {
            assert_eq!(root, fs::canonicalize(dir.path()).unwrap());
            assert_eq!(entry, "/app.html");
        }
        StartTarget::Url(_) => panic!("expected local file"),
    }
}

#[test]
fn resolve_target_uses_url_for_directory_without_index() {
    let dir = TestDir::new("resolve-missing-index");

    assert_eq!(
        resolve_target(dir.path().to_str().unwrap()),
        StartTarget::Url(dir.path().to_string_lossy().to_string())
    );
}

#[test]
fn resolve_target_uses_url_for_missing_file_url_and_missing_plain_path() {
    assert_eq!(
        resolve_target("FILE:///definitely/missing/index.html"),
        StartTarget::Url("FILE:///definitely/missing/index.html".to_string())
    );
    assert_eq!(
        resolve_target("/definitely/missing/index.html"),
        StartTarget::Url("/definitely/missing/index.html".to_string())
    );
}
