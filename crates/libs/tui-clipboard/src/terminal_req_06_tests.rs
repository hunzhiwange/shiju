use super::*;
use std::path::PathBuf;

#[test]
fn terminal_req_06_quoted_unicode_path() {
    assert_eq!(
        normalize_pasted_path("'/tmp/中文 🙂.png'"),
        Some(PathBuf::from("/tmp/中文 🙂.png"))
    );
    assert_eq!(normalize_pasted_path("\"/tmp/a b.png\""), Some(PathBuf::from("/tmp/a b.png")));
}

#[test]
#[cfg(not(windows))]
fn terminal_req_06_escaped_space_and_backslash() {
    assert_eq!(normalize_pasted_path(r"/tmp/a\ b.png"), Some(PathBuf::from("/tmp/a b.png")));
    assert_eq!(normalize_pasted_path(r"/tmp/a\\b.png"), Some(PathBuf::from(r"/tmp/a\b.png")));
}

#[test]
fn terminal_req_06_file_uri_decodes_unicode() {
    assert_eq!(
        normalize_pasted_path("file:///tmp/%E4%B8%AD%20a.png"),
        Some(PathBuf::from(if cfg!(windows) { "tmp/中 a.png" } else { "/tmp/中 a.png" }))
    );
}

#[test]
fn terminal_req_06_remote_and_invalid_uri_rejected() {
    for value in [
        "file://remote/tmp/a.png",
        "file:///tmp/%ZZ.png",
        "file:///tmp/%00.png",
        "file:///tmp/a.png#fragment",
    ] {
        assert!(normalize_pasted_path(value).is_none(), "{value}");
    }
}

#[test]
fn terminal_req_06_commands_and_multiple_paths_remain_text() {
    for value in [
        "cat /tmp/a.png",
        "/tmp/a.png /tmp/b.png",
        "$(touch marker).png",
        "/tmp/a.png\n",
        "/tmp/a.png\r\n/tmp/b.png",
    ] {
        assert!(image_path_attachment(value).is_none(), "{value}");
    }
}

#[test]
fn terminal_req_06_empty_missing_and_unclosed_quotes_remain_text() {
    for value in ["", "   ", "'unterminated.png", "/missing-terminal-req-06/file.png"] {
        assert!(image_path_attachment(value).is_none(), "{value}");
    }
}
