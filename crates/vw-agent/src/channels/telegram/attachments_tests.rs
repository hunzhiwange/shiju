use super::attachments::{
    IncomingAttachmentKind, TELEGRAM_MAX_FILE_DOWNLOAD_BYTES, TelegramAttachmentKind,
    format_attachment_content, infer_attachment_kind_from_target, is_http_url, is_image_extension,
    parse_attachment_markers, parse_path_only_attachment, resolve_workspace_attachment_output_path,
    resolve_workspace_attachment_path, sanitize_attachment_filename, sanitize_generated_extension,
};
use std::path::Path;

#[test]
fn attachment_kind_from_marker_accepts_aliases_and_rejects_unknown() {
    assert_eq!(TelegramAttachmentKind::from_marker(" photo "), Some(TelegramAttachmentKind::Image));
    assert_eq!(TelegramAttachmentKind::from_marker("IMAGE"), Some(TelegramAttachmentKind::Image));
    assert_eq!(
        TelegramAttachmentKind::from_marker("document"),
        Some(TelegramAttachmentKind::Document)
    );
    assert_eq!(TelegramAttachmentKind::from_marker("file"), Some(TelegramAttachmentKind::Document));
    assert_eq!(TelegramAttachmentKind::from_marker("video"), Some(TelegramAttachmentKind::Video));
    assert_eq!(TelegramAttachmentKind::from_marker("audio"), Some(TelegramAttachmentKind::Audio));
    assert_eq!(TelegramAttachmentKind::from_marker("voice"), Some(TelegramAttachmentKind::Voice));
    assert_eq!(TelegramAttachmentKind::from_marker("unknown"), None);
}

#[test]
fn image_extension_is_case_insensitive_and_rejects_non_images() {
    assert!(is_image_extension(Path::new("A.PNG")));
    assert!(is_image_extension(Path::new("photo.jpeg")));
    assert!(is_image_extension(Path::new("image.webp")));
    assert!(is_image_extension(Path::new("bitmap.bmp")));
    assert!(!is_image_extension(Path::new("notes.md")));
    assert!(!is_image_extension(Path::new("no-extension")));
}

#[test]
fn attachment_content_only_uses_image_marker_for_image_paths() {
    let image = format_attachment_content(
        IncomingAttachmentKind::Photo,
        "photo.jpg",
        Path::new("/tmp/photo.jpg"),
    );
    let text = format_attachment_content(
        IncomingAttachmentKind::Photo,
        "notes.md",
        Path::new("/tmp/notes.md"),
    );

    assert_eq!(image, "[IMAGE:/tmp/photo.jpg]");
    assert!(text.starts_with("[Document: notes.md]"));
}

#[test]
fn document_with_image_extension_formats_as_image_marker() {
    let content = format_attachment_content(
        IncomingAttachmentKind::Document,
        "scan.PNG",
        Path::new("/tmp/scan.PNG"),
    );

    assert_eq!(content, "[IMAGE:/tmp/scan.PNG]");
}

#[test]
fn attachment_filename_sanitizer_rejects_directory_markers_and_truncates() {
    assert_eq!(sanitize_attachment_filename("../report.pdf").as_deref(), Some("report.pdf"));
    assert_eq!(sanitize_attachment_filename(r"dir\report.pdf").as_deref(), Some(r"dir_report.pdf"));
    assert_eq!(sanitize_attachment_filename("  spaced.txt  ").as_deref(), Some("spaced.txt"));
    assert_eq!(sanitize_attachment_filename(".."), None);
    assert_eq!(sanitize_attachment_filename("."), None);
    assert_eq!(sanitize_attachment_filename(""), None);

    let long_name = format!("{}{}", "a".repeat(140), ".txt");
    let sanitized =
        sanitize_attachment_filename(&long_name).expect("long filename should sanitize");
    assert_eq!(sanitized.chars().count(), 128);
}

#[test]
fn generated_extension_sanitizer_filters_and_defaults() {
    assert_eq!(sanitize_generated_extension("Jp*g!VeryLong").as_str(), "jpgveryl");
    assert_eq!(sanitize_generated_extension("..."), "jpg");
    assert_eq!(sanitize_generated_extension("PNG"), "png");
}

#[test]
fn http_url_detection_is_scheme_specific() {
    assert!(is_http_url("https://example.test/file.png"));
    assert!(is_http_url("http://example.test/file.png"));
    assert!(!is_http_url("ftp://example.test/file.png"));
    assert!(!is_http_url("HTTPS://example.test/file.png"));
}

#[test]
fn infer_attachment_kind_from_target_maps_known_extensions() {
    assert_eq!(
        infer_attachment_kind_from_target("https://example.test/photo.JPG?size=large#frag"),
        Some(TelegramAttachmentKind::Image)
    );
    assert_eq!(
        infer_attachment_kind_from_target("movie.webm"),
        Some(TelegramAttachmentKind::Video)
    );
    assert_eq!(infer_attachment_kind_from_target("song.FLAC"), Some(TelegramAttachmentKind::Audio));
    assert_eq!(infer_attachment_kind_from_target("clip.opus"), Some(TelegramAttachmentKind::Voice));
    assert_eq!(
        infer_attachment_kind_from_target("report.PDF"),
        Some(TelegramAttachmentKind::Document)
    );
    assert_eq!(infer_attachment_kind_from_target("archive.unknown"), None);
    assert_eq!(infer_attachment_kind_from_target("no-extension"), None);
}

#[test]
fn path_only_attachment_accepts_urls_and_existing_files() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let image_path = tempdir.path().join("photo.jpg");
    std::fs::write(&image_path, b"image").expect("write image");

    let from_url = parse_path_only_attachment("`https://example.test/photo.png?x=1`")
        .expect("url should parse");
    assert_eq!(from_url.kind, TelegramAttachmentKind::Image);
    assert_eq!(from_url.target, "https://example.test/photo.png?x=1");

    let from_file = parse_path_only_attachment(&format!("\"file://{}\"", image_path.display()))
        .expect("file URL should parse");
    assert_eq!(from_file.kind, TelegramAttachmentKind::Image);
    assert_eq!(from_file.target, image_path.display().to_string());
}

#[test]
fn path_only_attachment_rejects_non_single_attachment_messages() {
    assert!(parse_path_only_attachment("").is_none());
    assert!(parse_path_only_attachment("photo.jpg with words").is_none());
    assert!(parse_path_only_attachment("one.png\ntwo.png").is_none());
    assert!(parse_path_only_attachment("/missing/file.pdf").is_none());
    assert!(parse_path_only_attachment("https://example.test/no-extension").is_none());
}

#[test]
fn marker_parser_extracts_known_markers_and_keeps_invalid_text() {
    let (cleaned, attachments) = parse_attachment_markers(
        "see [IMAGE:/tmp/a.png] and [video:https://example.test/v.mp4] [BAD:x] [FILE:  ]",
    );

    assert_eq!(cleaned, "see  and  [BAD:x] [FILE:  ]");
    assert_eq!(attachments.len(), 2);
    assert_eq!(attachments[0].kind, TelegramAttachmentKind::Image);
    assert_eq!(attachments[0].target, "/tmp/a.png");
    assert_eq!(attachments[1].kind, TelegramAttachmentKind::Video);
}

#[test]
fn marker_parser_handles_nested_and_unclosed_brackets() {
    let (cleaned, attachments) =
        parse_attachment_markers("before [IMAGE:path[inner].png] after [DOCUMENT:missing");

    assert_eq!(cleaned, "before  after [DOCUMENT:missing");
    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0].target, "path[inner].png");
}

#[test]
fn resolve_workspace_attachment_path_accepts_workspace_relative_forms() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let nested = tempdir.path().join("nested");
    std::fs::create_dir(&nested).expect("create nested");
    let file = nested.join("report.pdf");
    std::fs::write(&file, b"report").expect("write report");

    assert_eq!(
        resolve_workspace_attachment_path(tempdir.path(), "nested/report.pdf")
            .expect("relative path"),
        file.canonicalize().expect("canonical file")
    );
    assert_eq!(
        resolve_workspace_attachment_path(tempdir.path(), "/workspace/nested/report.pdf")
            .expect("workspace path"),
        file.canonicalize().expect("canonical file")
    );
    assert_eq!(
        resolve_workspace_attachment_path(tempdir.path(), &file.display().to_string())
            .expect("absolute path"),
        file.canonicalize().expect("canonical file")
    );
}

#[test]
fn resolve_workspace_attachment_path_rejects_bad_targets() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::NamedTempFile::new().expect("outside file");

    let null_err = resolve_workspace_attachment_path(tempdir.path(), "bad\0name")
        .expect_err("null byte should fail")
        .to_string();
    assert!(null_err.contains("null byte"));

    let missing_err = resolve_workspace_attachment_path(tempdir.path(), "missing.pdf")
        .expect_err("missing path should fail")
        .to_string();
    assert!(missing_err.contains("not found"));

    let dir_err = resolve_workspace_attachment_path(tempdir.path(), "/workspace")
        .expect_err("directory should fail")
        .to_string();
    assert!(dir_err.contains("not a file"));

    let escape_err =
        resolve_workspace_attachment_path(tempdir.path(), &outside.path().display().to_string())
            .expect_err("outside path should fail")
            .to_string();
    assert!(escape_err.contains("escapes workspace"));
}

#[cfg(unix)]
#[test]
fn resolve_workspace_attachment_path_rejects_symlink_escape() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::NamedTempFile::new().expect("outside file");
    let link = tempdir.path().join("link.pdf");
    std::os::unix::fs::symlink(outside.path(), &link).expect("create symlink");

    let err = resolve_workspace_attachment_path(tempdir.path(), "link.pdf")
        .expect_err("symlink escape should fail")
        .to_string();
    assert!(err.contains("escapes workspace"));
}

#[tokio::test]
async fn resolve_workspace_attachment_output_path_creates_safe_destination() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = resolve_workspace_attachment_output_path(tempdir.path(), "../report.pdf")
        .await
        .expect("output path");

    let expected_dir = tempdir.path().canonicalize().unwrap().join("telegram_files");
    assert!(path.starts_with(expected_dir));
    assert_eq!(path.file_name().and_then(|name| name.to_str()), Some("report.pdf"));
    assert!(path.parent().expect("parent").is_dir());
}

#[tokio::test]
async fn resolve_workspace_attachment_output_path_accepts_existing_regular_file() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let save_dir = tempdir.path().join("telegram_files");
    std::fs::create_dir_all(&save_dir).expect("create save dir");
    let existing = save_dir.join("report.pdf");
    std::fs::write(&existing, b"old report").expect("write existing file");

    let path = resolve_workspace_attachment_output_path(tempdir.path(), "report.pdf")
        .await
        .expect("existing regular file should remain a valid destination");

    assert_eq!(path, existing.canonicalize().expect("canonical existing file"));
    assert_eq!(std::fs::read(path).expect("read existing file"), b"old report");
}

#[tokio::test]
async fn resolve_workspace_attachment_output_path_rejects_invalid_or_unsafe_existing_path() {
    let tempdir = tempfile::tempdir().expect("tempdir");

    let invalid = resolve_workspace_attachment_output_path(tempdir.path(), "..")
        .await
        .expect_err("invalid filename");
    assert!(invalid.to_string().contains("invalid attachment filename"));

    let save_dir = tempdir.path().join("telegram_files");
    std::fs::create_dir_all(&save_dir).expect("create save dir");
    std::fs::create_dir(save_dir.join("dir-target")).expect("create target dir");

    let dir_err = resolve_workspace_attachment_output_path(tempdir.path(), "dir-target")
        .await
        .expect_err("directory target should fail")
        .to_string();
    assert!(dir_err.contains("not a regular file"));
}

#[cfg(unix)]
#[tokio::test]
async fn resolve_workspace_attachment_output_path_rejects_symlink_target() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let save_dir = tempdir.path().join("telegram_files");
    std::fs::create_dir_all(&save_dir).expect("create save dir");
    let outside = tempfile::NamedTempFile::new().expect("outside file");
    std::os::unix::fs::symlink(outside.path(), save_dir.join("linked.pdf"))
        .expect("create symlink");

    let err = resolve_workspace_attachment_output_path(tempdir.path(), "linked.pdf")
        .await
        .expect_err("symlink output should fail")
        .to_string();
    assert!(err.contains("refusing to write"));
}

#[test]
fn telegram_download_limit_is_twenty_mebibytes() {
    assert_eq!(TELEGRAM_MAX_FILE_DOWNLOAD_BYTES, 20 * 1024 * 1024);
}
// 本文件验证 Telegram 附件标记、类型推断、文件名清理与目标判定。
