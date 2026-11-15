use super::*;
use std::fs;
use tempfile::tempdir;
#[test]
fn sanitize_attachment_stem_replaces_unsafe_characters_and_falls_back() {
    assert_eq!(sanitize_attachment_stem(std::path::Path::new("screen shot!.png")), "screen-shot");
    assert_eq!(sanitize_attachment_stem(std::path::Path::new("你好.png")), "image");
    assert_eq!(sanitize_attachment_stem(std::path::Path::new("!!!.png")), "image");
}

#[test]
fn attachment_name_uses_file_name_or_display_path() {
    assert_eq!(attachment_name(std::path::Path::new("/tmp/capture.png")), "capture.png");
    assert_eq!(attachment_name(std::path::Path::new("/")), "/");
}

#[test]
fn path_within_dir_accepts_descendants_and_rejects_siblings_or_missing_paths() {
    let parent = tempdir().expect("parent tempdir");
    let sibling = tempdir().expect("sibling tempdir");
    let nested = parent.path().join("nested/file.txt");
    fs::create_dir_all(nested.parent().expect("nested parent")).expect("create nested parent");
    fs::write(&nested, "body").expect("write nested file");

    assert!(path_within_dir(&nested, parent.path()));
    assert!(!path_within_dir(&nested, sibling.path()));
    assert!(!path_within_dir(&parent.path().join("missing.txt"), parent.path()));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn clipboard_snapshot_path_is_content_addressed_png() {
    let snapshot_dir = tempdir().expect("snapshot tempdir");

    let first = build_clipboard_image_attachment_path(1, 1, &[1, 2, 3, 4], snapshot_dir.path());
    let repeated = build_clipboard_image_attachment_path(1, 1, &[1, 2, 3, 4], snapshot_dir.path());
    let changed = build_clipboard_image_attachment_path(1, 1, &[4, 3, 2, 1], snapshot_dir.path());

    assert_eq!(first, repeated);
    assert_ne!(first, changed);
    assert_eq!(first.extension().and_then(|value| value.to_str()), Some("png"));
    assert!(first.file_name().unwrap().to_string_lossy().starts_with("clipboard-"));
}

#[test]
fn external_image_is_copied_into_snapshot_dir() {
    let workspace_dir = tempdir().expect("workspace tempdir");
    let external_dir = tempdir().expect("external tempdir");
    let snapshot_dir = tempdir().expect("snapshot tempdir");
    let source = external_dir.path().join("screen shot.png");
    fs::write(&source, b"png-bytes").expect("write source image");
    let metadata = fs::metadata(&source).expect("metadata");

    let copied = stabilize_image_attachment(
        &source,
        &metadata,
        Some(workspace_dir.path()),
        snapshot_dir.path(),
    )
    .expect("stabilize image");

    assert_ne!(copied, source);
    assert!(copied.starts_with(snapshot_dir.path()));
    assert_eq!(fs::read(&copied).expect("read copied"), b"png-bytes");
}

#[test]
fn workspace_image_keeps_original_path() {
    let workspace_dir = tempdir().expect("workspace tempdir");
    let snapshot_dir = tempdir().expect("snapshot tempdir");
    let source = workspace_dir.path().join("diagram.png");
    fs::write(&source, b"workspace-image").expect("write workspace image");
    let metadata = fs::metadata(&source).expect("metadata");

    let kept = stabilize_image_attachment(
        &source,
        &metadata,
        Some(workspace_dir.path()),
        snapshot_dir.path(),
    )
    .expect("stabilize image");

    assert_eq!(kept, source);
}

#[test]
fn snapshot_path_is_stable_for_same_source_version() {
    let snapshot_dir = tempdir().expect("snapshot tempdir");
    let external_dir = tempdir().expect("external tempdir");
    let source = external_dir.path().join("capture.png");
    fs::write(&source, b"stable-bytes").expect("write source image");
    let metadata = fs::metadata(&source).expect("metadata");

    let first = build_image_attachment_snapshot_path(&source, &metadata, snapshot_dir.path());
    let second = build_image_attachment_snapshot_path(&source, &metadata, snapshot_dir.path());

    assert_eq!(first, second);
}

#[test]
fn snapshot_path_uses_img_extension_when_source_has_no_extension() {
    let snapshot_dir = tempdir().expect("snapshot tempdir");
    let external_dir = tempdir().expect("external tempdir");
    let source = external_dir.path().join("clipboard");
    fs::write(&source, b"raw-bytes").expect("write source image");
    let metadata = fs::metadata(&source).expect("metadata");

    let path = build_image_attachment_snapshot_path(&source, &metadata, snapshot_dir.path());

    assert_eq!(path.extension().and_then(|value| value.to_str()), Some("img"));
    assert!(path.file_name().unwrap().to_string_lossy().starts_with("clipboard-"));
}

#[test]
fn image_already_inside_snapshot_dir_keeps_original_path() {
    let snapshot_dir = tempdir().expect("snapshot tempdir");
    let source = snapshot_dir.path().join("existing.png");
    fs::write(&source, b"snapshot-image").expect("write snapshot image");
    let metadata = fs::metadata(&source).expect("metadata");

    let kept =
        stabilize_image_attachment(&source, &metadata, None, snapshot_dir.path()).expect("kept");

    assert_eq!(kept, source);
}

#[test]
fn stabilize_image_attachment_reports_snapshot_directory_creation_failure() {
    let external_dir = tempdir().expect("external tempdir");
    let source = external_dir.path().join("capture.png");
    let invalid_snapshot_dir = external_dir.path().join("not-a-directory");
    fs::write(&source, b"png-bytes").expect("write source image");
    fs::write(&invalid_snapshot_dir, b"occupied").expect("write blocking file");
    let metadata = fs::metadata(&source).expect("metadata");

    let error = stabilize_image_attachment(&source, &metadata, None, &invalid_snapshot_dir)
        .expect_err("file cannot be used as snapshot directory");

    assert!(error.fallback.contains("创建图片附件目录失败"));
    assert!(error.fallback.contains(&invalid_snapshot_dir.display().to_string()));
}
