use std::fs;

use super::attachments_outgoing::{
    DiscordAttachment, DiscordAttachmentKind, classify_outgoing_attachments,
    parse_attachment_markers, resolve_local_attachment_path, with_inline_attachment_urls,
};

#[test]
fn attachment_kind_markers_support_all_names_and_aliases() {
    assert_eq!(DiscordAttachmentKind::from_marker(" image "), Some(DiscordAttachmentKind::Image));
    assert_eq!(DiscordAttachmentKind::from_marker("PHOTO"), Some(DiscordAttachmentKind::Image));
    assert_eq!(
        DiscordAttachmentKind::from_marker("document"),
        Some(DiscordAttachmentKind::Document)
    );
    assert_eq!(DiscordAttachmentKind::from_marker("FILE"), Some(DiscordAttachmentKind::Document));
    assert_eq!(DiscordAttachmentKind::from_marker("video"), Some(DiscordAttachmentKind::Video));
    assert_eq!(DiscordAttachmentKind::from_marker("audio"), Some(DiscordAttachmentKind::Audio));
    assert_eq!(DiscordAttachmentKind::from_marker("voice"), Some(DiscordAttachmentKind::Voice));
    assert_eq!(DiscordAttachmentKind::from_marker("unknown"), None);

    assert_eq!(DiscordAttachmentKind::Image.marker_name(), "IMAGE");
    assert_eq!(DiscordAttachmentKind::Document.marker_name(), "DOCUMENT");
    assert_eq!(DiscordAttachmentKind::Video.marker_name(), "VIDEO");
    assert_eq!(DiscordAttachmentKind::Audio.marker_name(), "AUDIO");
    assert_eq!(DiscordAttachmentKind::Voice.marker_name(), "VOICE");
}

#[test]
fn attachment_marker_parsing_removes_valid_markers_and_keeps_invalid_text() {
    let (text, attachments) =
        parse_attachment_markers("hello [IMAGE:/workspace/a.png] [UNKNOWN:x] [FILE: report.pdf ]");

    assert_eq!(text, "hello  [UNKNOWN:x]");
    assert_eq!(attachments.len(), 2);
    assert_eq!(attachments[0].kind, DiscordAttachmentKind::Image);
    assert_eq!(attachments[1].kind.marker_name(), "DOCUMENT");
}

#[test]
fn attachment_marker_parsing_returns_trimmed_plain_text_without_markers() {
    let (text, attachments) = parse_attachment_markers("  plain message  ");

    assert_eq!(text, "plain message");
    assert!(attachments.is_empty());
}

#[test]
fn attachment_marker_parsing_keeps_markers_without_separator() {
    let (text, attachments) = parse_attachment_markers("body [IMAGE] tail");

    assert_eq!(text, "body [IMAGE] tail");
    assert!(attachments.is_empty());
}

#[test]
fn outgoing_attachments_are_classified_by_url_scheme() {
    let attachments = vec![
        DiscordAttachment {
            kind: DiscordAttachmentKind::Image,
            target: "https://example.test/a.png".to_string(),
        },
        DiscordAttachment {
            kind: DiscordAttachmentKind::Document,
            target: "local.txt".to_string(),
        },
    ];

    let (local, remote, unresolved) = classify_outgoing_attachments(&attachments);

    assert_eq!(local.len(), 1);
    assert_eq!(remote, vec!["https://example.test/a.png"]);
    assert!(unresolved.is_empty());
    assert_eq!(
        with_inline_attachment_urls("body", &remote, &[]),
        "body\nhttps://example.test/a.png"
    );
}

#[test]
fn outgoing_attachments_trim_remote_urls_and_keep_local_targets() {
    let attachments = vec![
        DiscordAttachment {
            kind: DiscordAttachmentKind::Audio,
            target: " http://example.test/a.mp3 ".to_string(),
        },
        DiscordAttachment { kind: DiscordAttachmentKind::Video, target: " local.mp4 ".to_string() },
    ];

    let (local, remote, unresolved) = classify_outgoing_attachments(&attachments);

    assert_eq!(remote, vec!["http://example.test/a.mp3"]);
    assert_eq!(local, vec![attachments[1].clone()]);
    assert!(unresolved.is_empty());
}

#[test]
fn outgoing_attachment_classification_handles_empty_input() {
    let (local, remote, unresolved) = classify_outgoing_attachments(&[]);

    assert!(local.is_empty());
    assert!(remote.is_empty());
    assert!(unresolved.is_empty());
}

#[test]
fn resolve_local_attachment_path_rejects_missing_workspace_and_path_escape() {
    assert!(resolve_local_attachment_path(None, "a.txt").is_err());

    let workspace = tempfile::tempdir().expect("workspace should exist");
    let inside = workspace.path().join("a.txt");
    fs::write(&inside, "ok").expect("file should be written");
    let resolved = resolve_local_attachment_path(Some(&workspace.path().to_path_buf()), "a.txt")
        .expect("inside file should resolve");

    assert_eq!(resolved, inside.canonicalize().unwrap());
    assert!(
        resolve_local_attachment_path(Some(&workspace.path().to_path_buf()), "../a.txt").is_err()
    );
}

#[test]
fn attachment_marker_parsing_supports_aliases_and_unclosed_markers() {
    let (text, attachments) = parse_attachment_markers(
        "before [photo: https://cdn.test/p.png ] [VOICE:voice.ogg] [IMAGE:] [FILE:missing",
    );

    assert_eq!(text, "before   [IMAGE:] [FILE:missing");
    assert_eq!(attachments.len(), 2);
    assert_eq!(attachments[0].kind, DiscordAttachmentKind::Image);
    assert_eq!(attachments[0].target, "https://cdn.test/p.png");
    assert_eq!(attachments[1].kind, DiscordAttachmentKind::Voice);
    assert_eq!(attachments[1].kind.marker_name(), "VOICE");
}

#[test]
fn with_inline_attachment_urls_omits_blank_content_and_appends_unresolved() {
    let remote = vec!["https://cdn.test/a.png".to_string()];
    let unresolved = vec!["[FILE:missing.pdf]".to_string()];

    assert_eq!(
        with_inline_attachment_urls("   ", &remote, &unresolved),
        "https://cdn.test/a.png\n[FILE:missing.pdf]"
    );
}

#[test]
fn with_inline_attachment_urls_returns_empty_when_all_parts_are_empty() {
    assert_eq!(with_inline_attachment_urls("", &[], &[]), "");
}

#[test]
fn resolve_local_attachment_path_supports_workspace_prefix_and_rejects_directories() {
    let workspace = tempfile::tempdir().expect("workspace should exist");
    let nested = workspace.path().join("nested");
    fs::create_dir(&nested).unwrap();
    let file = nested.join("report.txt");
    fs::write(&file, "ok").unwrap();
    let workspace_path = workspace.path().to_path_buf();

    let resolved =
        resolve_local_attachment_path(Some(&workspace_path), "/workspace/nested/report.txt")
            .expect("workspace-prefixed file should resolve");
    assert_eq!(resolved, file.canonicalize().unwrap());

    let error =
        resolve_local_attachment_path(Some(&workspace_path), "/workspace").unwrap_err().to_string();
    assert!(error.contains("not a file"));
}

#[test]
fn resolve_local_attachment_path_supports_absolute_inside_path_and_missing_files() {
    let workspace = tempfile::tempdir().expect("workspace should exist");
    let file = workspace.path().join("absolute.txt");
    fs::write(&file, "ok").unwrap();
    let workspace_path = workspace.path().to_path_buf();

    let resolved =
        resolve_local_attachment_path(Some(&workspace_path), file.to_string_lossy().as_ref())
            .expect("absolute file inside workspace should resolve");
    assert_eq!(resolved, file.canonicalize().unwrap());

    let error = resolve_local_attachment_path(Some(&workspace_path), "missing.txt")
        .unwrap_err()
        .to_string();
    assert!(error.contains("attachment path not found"));
}

#[test]
fn resolve_local_attachment_path_reports_missing_target_for_stale_workspace() {
    let workspace = tempfile::tempdir().expect("workspace should exist");
    let stale_workspace = workspace.path().to_path_buf();
    drop(workspace);

    let error = resolve_local_attachment_path(Some(&stale_workspace), "missing.txt")
        .unwrap_err()
        .to_string();

    assert!(error.contains("attachment path not found: missing.txt"));
}
// 本文件验证 Discord 出站附件标记解析、类型判定与本地路径安全边界。
