use super::super::attachments_outgoing as discord_attachments_outgoing;
use super::*;

/// 测试解析附件标记提取支持的标记
/// 应该正确识别 [IMAGE:...] 和 [DOCUMENT:...] 标记
#[test]
fn parse_attachment_markers_extracts_supported_markers() {
    let input = "Report\n[IMAGE:https://example.com/a.png]\n[DOCUMENT:/tmp/a.pdf]";
    let (cleaned, attachments) = discord_attachments_outgoing::parse_attachment_markers(input);

    assert_eq!(cleaned, "Report");
    assert_eq!(attachments.len(), 2);
    assert_eq!(attachments[0].kind, discord_attachments_outgoing::DiscordAttachmentKind::Image);
    assert_eq!(attachments[0].target, "https://example.com/a.png");
    assert_eq!(attachments[1].kind, discord_attachments_outgoing::DiscordAttachmentKind::Document);
    assert_eq!(attachments[1].target, "/tmp/a.pdf");
}

/// 测试解析附件标记保留无效标记文本
/// 无法识别的标记格式应该保留在文本中
#[test]
fn parse_attachment_markers_keeps_invalid_marker_text() {
    let input = "Hello [NOT_A_MARKER:foo] world";
    let (cleaned, attachments) = discord_attachments_outgoing::parse_attachment_markers(input);

    assert_eq!(cleaned, input);
    assert!(attachments.is_empty());
}

/// 测试分类传出附件：区分本地、远程和未解析路径
/// 应该正确分类本地文件、远程 URL 和不存在的路径
#[test]
fn classify_outgoing_attachments_splits_local_remote_and_unresolved() {
    let temp = tempfile::tempdir().expect("tempdir");
    let file_path = temp.path().join("image.png");
    std::fs::write(&file_path, b"fake").expect("写入测试文件");

    let attachments = vec![
        discord_attachments_outgoing::DiscordAttachment {
            kind: discord_attachments_outgoing::DiscordAttachmentKind::Image,
            target: file_path.to_string_lossy().to_string(),
        },
        discord_attachments_outgoing::DiscordAttachment {
            kind: discord_attachments_outgoing::DiscordAttachmentKind::Image,
            target: "https://example.com/remote.png".to_string(),
        },
        discord_attachments_outgoing::DiscordAttachment {
            kind: discord_attachments_outgoing::DiscordAttachmentKind::Video,
            target: "/tmp/does-not-exist.mp4".to_string(),
        },
    ];

    let (locals, remotes, unresolved) =
        discord_attachments_outgoing::classify_outgoing_attachments(&attachments);
    assert_eq!(locals.len(), 2);
    assert_eq!(locals[0].target, file_path.to_string_lossy());
    assert_eq!(locals[1].target, "/tmp/does-not-exist.mp4");
    assert_eq!(remotes, vec!["https://example.com/remote.png".to_string()]);
    assert!(unresolved.is_empty());
}

/// 测试内联附件 URL 追加 URL 和未解析标记
/// 远程 URL 和未解析的标记应该追加到内容后面
#[test]
fn with_inline_attachment_urls_appends_urls_and_unresolved_markers() {
    let content = "Done";
    let remote_urls = vec!["https://example.com/a.png".to_string()];
    let unresolved = vec!["[IMAGE:/tmp/missing.png]".to_string()];

    let rendered = discord_attachments_outgoing::with_inline_attachment_urls(
        content,
        &remote_urls,
        &unresolved,
    );
    assert_eq!(rendered, "Done\nhttps://example.com/a.png\n[IMAGE:/tmp/missing.png]");
}

/// 测试解析本地附件路径阻止工作区逃逸
/// 不应该允许访问工作区目录之外的文件（路径遍历防护）
#[test]
fn resolve_local_attachment_path_blocks_workspace_escape() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("工作区应该存在");

    let outside = temp.path().join("outside.txt");
    std::fs::write(&outside, b"secret").expect("测试文件应该被写入");

    let channel = DiscordChannel::new("fake".into(), None, vec![], false, false)
        .with_workspace_dir(workspace.clone());

    let allowed_path = workspace.join("ok.txt");
    std::fs::write(&allowed_path, b"ok").expect("工作区测试文件应该被写入");
    let allowed = discord_attachments_outgoing::resolve_local_attachment_path(
        channel.workspace_dir.as_ref(),
        "ok.txt",
    )
    .expect("工作区文件应该被允许");
    assert!(allowed.starts_with(workspace.canonicalize().unwrap_or(workspace)));

    let escaped = discord_attachments_outgoing::resolve_local_attachment_path(
        channel.workspace_dir.as_ref(),
        outside.to_string_lossy().as_ref(),
    );
    assert!(escaped.is_err(), "工作区外的路径必须被拒绝");
}
