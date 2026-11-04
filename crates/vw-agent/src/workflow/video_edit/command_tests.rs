use super::*;
use crate::tools::ToolRuntimeContext;

#[test]
fn video_edit_command_parses_quoted_chinese_and_windows_paths() {
    let parsed =
        parse_video_edit_command(r#"/shiju 剪辑 "中文 空格.mp4" 保留重点"#).unwrap().unwrap();
    assert_eq!(parsed.video_path.to_str(), Some("中文 空格.mp4"));
    assert_eq!(parsed.requirements.as_deref(), Some("保留重点"));
    let parsed =
        parse_video_edit_command(r#"/shiju 剪辑 "C:\视频 文件\片段.mp4""#).unwrap().unwrap();
    assert_eq!(parsed.video_path.to_str(), Some(r"C:\视频 文件\片段.mp4"));
}

#[test]
fn video_edit_command_validates_real_paths_without_binding() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("中文 空格.mp4");
    std::fs::write(&file, b"not a video: cloud probe must validate").unwrap();
    let ctx = ToolRuntimeContext::new("video", Some(dir.path().to_string_lossy().into()));
    for path in [file.to_string_lossy().into_owned(), "中文 空格.mp4".into()] {
        let query = format!("/shiju 剪辑 \"{path}\" 要求");
        let req = prepare_video_edit_request(&query, &ctx).unwrap().unwrap();
        assert_eq!(req.video_path, file.canonicalize().unwrap());
        assert_eq!(req.original_request, query);
        assert_eq!(req.requirements.as_deref(), Some("要求"));
    }
}

#[test]
fn video_edit_command_rejects_invalid_files_and_ambiguous_input() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("empty.mp4"), b"").unwrap();
    std::fs::write(dir.path().join("text.txt"), b"text").unwrap();
    let ctx = ToolRuntimeContext::new("video", Some(dir.path().to_string_lossy().into()));
    for path in ["missing.mp4", "empty.mp4", "text.txt", "."] {
        assert!(
            prepare_video_edit_request(&format!("/shiju 剪辑 {path}"), &ctx).is_err(),
            "{path}"
        );
    }
    for query in ["/shiju 剪辑", "/shiju 剪辑 \"open.mp4", "/shiju 剪辑 a.mp4 b.mp4"] {
        assert!(parse_video_edit_command(query).is_err(), "{query}");
    }
    assert!(parse_video_edit_command("/shiju 搜索 a.mp4").unwrap().is_none());
    assert!(parse_video_edit_command("/shiju 剪辑器 a.mp4").unwrap().is_none());
}

#[cfg(unix)]
#[test]
fn video_edit_command_symlink_obeys_native_security_and_full_access() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let file = external.path().join("source.mp4");
    std::fs::write(&file, b"probe later").unwrap();
    std::os::unix::fs::symlink(&file, root.path().join("link.mp4")).unwrap();
    let ctx = ToolRuntimeContext::new("video", Some(root.path().to_string_lossy().into()));
    let mut config = crate::app::agent::config::Config::default();
    config.autonomy.workspace_only = true;
    let ctx = ctx.with_test_config(config);
    assert!(prepare_video_edit_request("/shiju 剪辑 link.mp4", &ctx).is_err());
    let full = ctx.clone().with_tool_use_context(
        ctx.tool_use_context().as_ref().clone().with_full_access_enabled(true),
    );
    assert_eq!(
        prepare_video_edit_request("/shiju 剪辑 link.mp4", &full).unwrap().unwrap().video_path,
        file.canonicalize().unwrap()
    );
}

#[test]
fn video_edit_command_shell_characters_remain_literal() {
    let root = tempfile::tempdir().unwrap();
    let name = "$(touch PWNED);`id`.mp4";
    std::fs::write(root.path().join(name), b"probe later").unwrap();
    let ctx = ToolRuntimeContext::new("video", Some(root.path().to_string_lossy().into()));
    assert!(
        prepare_video_edit_request(&format!("/shiju 剪辑 \"{name}\""), &ctx).unwrap().is_some()
    );
    assert!(!root.path().join("PWNED").exists());
}

#[test]
fn video_edit_command_external_allowed_root_and_no_project() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let file = external.path().join("source.mp4");
    std::fs::write(&file, b"cloud probe pending").unwrap();
    let mut config = crate::app::agent::config::Config::default();
    config.workspace_dir = root.path().to_path_buf();
    config.autonomy.workspace_only = true;
    config.autonomy.allowed_roots = vec![external.path().to_string_lossy().into_owned()];
    let ctx = ToolRuntimeContext::new("video", Some(root.path().to_string_lossy().into()))
        .with_test_config(config);
    let query = format!("/shiju 剪辑 '{}'", file.display());
    assert!(prepare_video_edit_request(&query, &ctx).unwrap().is_some());
    let full = ToolRuntimeContext::new("video", None).with_tool_use_context(
        crate::tools::ToolUseContext::new("video", None).with_full_access_enabled(true),
    );
    let req = prepare_video_edit_request(&query, &full).unwrap().unwrap();
    assert_eq!(req.project_root, None);
    assert!(prepare_video_edit_request("/shiju 剪辑 relative.mp4", &full).is_err());
}

#[test]
fn video_edit_command_delegated_parent_remains_restricted_under_full_access() {
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let file = external.path().join("source.mp4");
    std::fs::write(&file, b"probe later").unwrap();
    let mut config = crate::app::agent::config::Config::default();
    config.autonomy.workspace_only = true;
    let policy = crate::app::agent::security::SecurityPolicy::from_configs(
        &config.autonomy,
        &config.security,
        root.path(),
    );
    let base =
        crate::tools::ToolUseContext::new("video", Some(root.path().to_string_lossy().into()))
            .with_security(Arc::new(policy))
            .for_child("child".into(), vec![])
            .with_full_access_enabled(true);
    let ctx = ToolRuntimeContext::new("child", Some(root.path().to_string_lossy().into()))
        .with_tool_use_context(base)
        .with_test_config(config);
    assert!(
        prepare_video_edit_request(&format!("/shiju 剪辑 '{}'", file.display()), &ctx).is_err()
    );
}
